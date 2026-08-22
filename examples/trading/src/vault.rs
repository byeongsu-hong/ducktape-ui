//! Custody for a build the Secure Enclave will not serve.
//!
//! The Enclave and the data-protection keychain serve signed code carrying
//! keychain entitlements, and a binary `cargo` built and nobody signed carries
//! neither: the first key it tries to make comes back `-34018` and the import
//! refuses rather than storing a wallet it cannot seal. `session.rs` says why
//! at length, and `scripts/sign-dev.sh` is the fix. This module is the other
//! answer, for a machine that is not going to be signed: a file this app
//! encrypts itself, opened by a passphrase the owner types.
//!
//! **What it is and is not.** The keychain's protection is the platform's
//! judgement about who may read an item, enforced by hardware. This is one
//! passphrase and a key derivation, enforced by arithmetic. It is strictly
//! weaker, and the panel says so rather than letting a reader believe the
//! guarantee did not change. What it is *not* is the option `README.md` records
//! as considered and rejected — "unsealed anyway, labelled", which would put the
//! account's own key in a keychain item at the moment the app has just proven it
//! cannot protect it. Nothing here is written in the clear.
//!
//! **The scheme**, and every part of it is `ring`'s, which is already in this
//! tree under rustls:
//!
//! - PBKDF2-HMAC-SHA512 over the passphrase, `ROUNDS` iterations, a 16-byte
//!   salt made once per file by the OS generator.
//! - ChaCha20-Poly1305 over each item, a fresh 12-byte nonce per write, and the
//!   item's own name as associated data — so a blob cannot be moved from one
//!   account's slot to another's and still open.
//! - A wrong passphrase is a tag that does not verify, which is
//!   indistinguishable from a corrupted file and is reported as the retryable
//!   refusal it is rather than as a broken keystore.
//!
//! The nonce is random rather than counted because the alternative is a counter
//! that has to survive a file being copied, and 96 random bits per write under
//! one key is the case the construction is specified for at this volume: this
//! file holds one wallet and four trading keys and is rewritten by hand.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use ring::aead::{Aad, LessSafeKey, NONCE_LEN, Nonce, UnboundKey};
use ring::{aead, pbkdf2};
use serde_json::{Value, json};

use crate::session::{Held, KeystoreError, Secret};

/// What the file says it is. A second scheme is told from this one by reading
/// rather than by guessing from the shape of what is inside.
const SCHEME: &str = "ducktape-passphrase-1";

/// Iterations of the derivation. `ring` names no default and the number is a
/// policy rather than a fact: this is OWASP's 2023 floor for PBKDF2-HMAC-SHA512,
/// and it is paid once per act — a store, an enrolment, an unlock — rather than
/// per item read.
const ROUNDS: u32 = 210_000;

const SALT_LEN: usize = 16;

/// Where the file lives when nobody said otherwise.
///
/// `cfg!(test)` sends it to the temp directory for the reason `export_dir` does:
/// a suite run must not write into the reader's own folders. Every function
/// here also has an `_in` form taking the directory, which is what the tests
/// below actually use, so two of them cannot collide on one path.
pub fn dir() -> PathBuf {
    if cfg!(test) {
        return std::env::temp_dir().join("dev.ducktape.trading-test");
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    if cfg!(target_os = "macos") {
        home.join("Library/Application Support/dev.ducktape.trading")
    } else {
        home.join(".local/share/dev.ducktape.trading")
    }
}

fn path(dir: &Path) -> PathBuf {
    dir.join("keys.json")
}

/// Whether this account has something in the file. Answered without the
/// passphrase, because it is the question that decides whether to *ask* for one.
pub fn holds(account: &str) -> bool {
    holds_in(&dir(), account)
}

pub fn holds_in(dir: &Path, account: &str) -> bool {
    read(dir).is_ok_and(|file| file.items.contains_key(account))
}

/// Whether the file holds anything at all, which is what a boot asks to know
/// whether this machine has been down this road before.
pub fn occupied_in(dir: &Path) -> bool {
    read(dir).is_ok_and(|file| !file.items.is_empty())
}

pub fn occupied() -> bool {
    occupied_in(&dir())
}

/// Seal a secret into the file under this account.
pub fn keep(account: &str, secret: &Secret, phrase: &str) -> Result<(), KeystoreError> {
    keep_in(&dir(), account, secret, phrase)
}

pub fn keep_in(
    dir: &Path,
    account: &str,
    secret: &Secret,
    phrase: &str,
) -> Result<(), KeystoreError> {
    if phrase.is_empty() {
        return Err(KeystoreError::wanting(NO_PASSPHRASE.to_owned()));
    }
    // A first run makes one. A file that is *there* and will not parse does
    // not: replacing it would be this app deleting an account key because it
    // could not read a line of JSON, and the reader would find out by unlocking
    // into "no key on this Mac". The refusal names the file, which is the same
    // advice a wrong passphrase gets.
    let mut file = match read(dir) {
        Ok(file) => file,
        Err(_) if !path(dir).exists() => Vault::new(fresh(SALT_LEN)),
        Err(failure) => return Err(failure),
    };
    // A file that already exists keeps its salt: every item in it was sealed
    // under the key that salt derives, and rolling it would strand them.
    let key = derive(phrase, &file.salt, file.rounds)?;
    // A passphrase that cannot open what is already here is a typo, and writing
    // under it would leave one file holding items from two passphrases — half
    // of which the owner can no longer open and nothing on screen saying which
    // half. Checked against any one item, because they all share the key.
    if let Some((name, blob)) = file.items.iter().next()
        && unseal(&key, name, blob).is_none()
    {
        return Err(KeystoreError::refused(wrong_passphrase(dir)));
    }
    file.items
        .insert(account.to_owned(), seal(&key, account, secret.expose())?);
    write(dir, &file)
}

/// Read one back. `Declined` rather than an error for a passphrase that does
/// not open it: the owner mistyping is a thing to ask again about, and the
/// session model already draws that differently from a keystore that is broken.
pub fn take(account: &str, phrase: &str) -> Result<Held, KeystoreError> {
    take_in(&dir(), account, phrase)
}

pub fn take_in(dir: &Path, account: &str, phrase: &str) -> Result<Held, KeystoreError> {
    // No file is `Missing` — nothing has been stored here, which is an answer.
    // A file that will not parse is not: reading it as "nothing" would tell a
    // reader whose keys are sitting right there to enrol again.
    let file = match read(dir) {
        Ok(file) => file,
        Err(_) if !path(dir).exists() => return Ok(Held::Missing),
        Err(failure) => return Err(failure),
    };
    let Some(blob) = file.items.get(account) else {
        return Ok(Held::Missing);
    };
    if phrase.is_empty() {
        return Err(KeystoreError::wanting(NO_PASSPHRASE.to_owned()));
    }
    let key = derive(phrase, &file.salt, file.rounds)?;
    match unseal(&key, account, blob) {
        Some(plain) => Ok(Held::Secret(plain)),
        // Not `Held::Declined`. A decline is a sheet the owner cancelled, and
        // the sentence the session model has for one says so — "Touch ID was
        // cancelled" over a build that raised no sheet is the panel describing
        // an app this is not. Retryable all the same, which is what `Refused`
        // carries.
        None => Err(KeystoreError::refused(wrong_passphrase(dir))),
    }
}

pub const NO_PASSPHRASE: &str = "This build cannot reach the Secure Enclave, so this app keeps \
                                 keys in a file it encrypts itself. Choose a passphrase for it \
                                 below — it is what opens the file, and nothing on this machine \
                                 can recover it for you.";

/// The refusal names the file, because a forgotten passphrase has exactly one
/// way out and it is not in this app: the file is the only thing holding these
/// keys, so deleting it is starting over, and every key in it goes with it. An
/// in-app button for that would be one press between a reader and their own
/// account — the path is the honest amount of friction.
fn wrong_passphrase(dir: &Path) -> String {
    format!(
        "That passphrase does not open {}. Nothing has changed. If it is lost, that file is the \
         only thing holding these keys — delete it to start again, and everything in it is gone.",
        path(dir).display()
    )
}

// ------------------------------------------------------------------- the file

struct Vault {
    salt: Vec<u8>,
    rounds: u32,
    items: BTreeMap<String, Vec<u8>>,
}

impl Vault {
    fn new(salt: Vec<u8>) -> Self {
        Self {
            salt,
            rounds: ROUNDS,
            items: BTreeMap::new(),
        }
    }
}

fn read(dir: &Path) -> Result<Vault, KeystoreError> {
    let target = path(dir);
    let text = std::fs::read_to_string(&target).map_err(|cause| {
        KeystoreError::plain(format!("could not read {}: {cause}", target.display()))
    })?;
    let parsed: Value = serde_json::from_str(&text).map_err(|cause| {
        KeystoreError::plain(format!("{} is not readable: {cause}", target.display()))
    })?;
    let unreadable = || {
        KeystoreError::plain(format!(
            "{} is not a key file this app wrote",
            target.display()
        ))
    };
    if parsed.get("scheme").and_then(Value::as_str) != Some(SCHEME) {
        return Err(unreadable());
    }
    let salt = parsed
        .get("salt")
        .and_then(Value::as_str)
        .and_then(|text| BASE64.decode(text).ok())
        .ok_or_else(unreadable)?;
    let rounds = parsed
        .get("rounds")
        .and_then(Value::as_u64)
        .and_then(|rounds| u32::try_from(rounds).ok())
        .filter(|rounds| *rounds > 0)
        .ok_or_else(unreadable)?;
    let items = parsed
        .get("items")
        .and_then(Value::as_object)
        .ok_or_else(unreadable)?
        .iter()
        .map(|(name, blob)| {
            blob.as_str()
                .and_then(|text| BASE64.decode(text).ok())
                .map(|bytes| (name.clone(), bytes))
                .ok_or_else(unreadable)
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    Ok(Vault {
        salt,
        rounds,
        items,
    })
}

fn write(dir: &Path, file: &Vault) -> Result<(), KeystoreError> {
    make_dir(dir).map_err(|cause| {
        KeystoreError::plain(format!("could not make {}: {cause}", dir.display()))
    })?;
    let body = json!({
        "scheme": SCHEME,
        "salt": BASE64.encode(&file.salt),
        "rounds": file.rounds,
        "items": file
            .items
            .iter()
            .map(|(name, blob)| (name.clone(), Value::String(BASE64.encode(blob))))
            .collect::<serde_json::Map<_, _>>(),
    });
    let target = path(dir);
    // Written beside and renamed over, so a crash halfway through leaves the
    // previous file rather than a truncated one nothing can open. The whole of
    // this app's custody can be in here.
    let staged = target.with_extension("json.new");
    write_private(&staged, &format!("{body:#}\n")).map_err(|cause| {
        KeystoreError::plain(format!("could not write {}: {cause}", staged.display()))
    })?;
    // The staged file was created at 0600, and a rename moves the inode rather
    // than the contents — so the mode arrives with it, and there is never a
    // moment where the real file exists, holds the blob and is readable.
    std::fs::rename(&staged, &target).map_err(|cause| {
        KeystoreError::plain(format!("could not replace {}: {cause}", target.display()))
    })
}

/// The directory, owner-only.
///
/// `mode` applies to what this call creates and leaves an existing directory
/// alone, which is right: on macOS the parent is `~/Library/Application
/// Support`, which belongs to the system rather than to this app.
#[cfg(unix)]
fn make_dir(dir: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;

    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
}

#[cfg(not(unix))]
fn make_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)
}

/// Write a file nobody else on this machine may read.
///
/// What is in it is ciphertext, so this is not the protection — the passphrase
/// and 210,000 rounds are. It is the difference between an attacker needing to
/// be *this* user and needing only to be *a* user: a plain `std::fs::write`
/// lands at 0644 under the usual umask, which hands every other local account a
/// copy to work on offline, at their leisure, leaving no trace in this app.
///
/// The mode has to be set at creation rather than after, or there is a window
/// where the file exists, holds the blob and is readable. A stale staging file
/// is removed first because `mode` applies only to a file this call creates,
/// and a leftover from a crashed write would otherwise keep whatever mode it
/// had.
#[cfg(unix)]
fn write_private(path: &Path, body: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => {}
        Err(cause) => return Err(cause),
    }
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?
        .write_all(body.as_bytes())
}

/// Windows has no mode to set here and its ACLs are a different design. The
/// road into this module is a Mac refusing an unsigned binary, so nothing
/// reaches it there — left as the plain write rather than as a wrong-shaped
/// imitation of the one above.
#[cfg(not(unix))]
fn write_private(path: &Path, body: &str) -> std::io::Result<()> {
    std::fs::write(path, body)
}

// ------------------------------------------------------------------ the sealing

fn fresh(len: usize) -> Vec<u8> {
    let mut bytes = vec![0_u8; len];
    // The same call a wallet and a trading key are minted with, and it fails
    // loudly for the same reason: a salt or a nonce from a weaker source is a
    // seal whose strength nobody can state.
    getrandom::fill(&mut bytes).expect("the OS generator");
    bytes
}

fn derive(phrase: &str, salt: &[u8], rounds: u32) -> Result<LessSafeKey, KeystoreError> {
    let rounds = std::num::NonZeroU32::new(rounds)
        .ok_or_else(|| KeystoreError::plain("the key file asks for no work at all".to_owned()))?;
    let mut key = [0_u8; 32];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA512,
        rounds,
        salt,
        phrase.as_bytes(),
        &mut key,
    );
    let unbound = UnboundKey::new(&aead::CHACHA20_POLY1305, &key)
        .map_err(|_| KeystoreError::plain("the derived key is the wrong length".to_owned()))?;
    // In place. `Secret::new(key.to_vec())` would have wiped a *copy* and left
    // this one on the stack, which is the shape of that mitigation that does
    // nothing. `black_box` is what stops the write being elided; the same pair
    // `from_raw_key` uses in `custody.rs`.
    key.fill(0);
    std::hint::black_box(&mut key);
    Ok(LessSafeKey::new(unbound))
}

fn seal(key: &LessSafeKey, account: &str, plain: &[u8]) -> Result<Vec<u8>, KeystoreError> {
    let nonce = fresh(NONCE_LEN);
    let mut blob = plain.to_vec();
    key.seal_in_place_append_tag(
        Nonce::try_assume_unique_for_key(&nonce)
            .map_err(|_| KeystoreError::plain("the nonce is the wrong length".to_owned()))?,
        Aad::from(account.as_bytes()),
        &mut blob,
    )
    .map_err(|_| KeystoreError::plain("the secret could not be sealed".to_owned()))?;
    let mut out = nonce;
    out.append(&mut blob);
    Ok(out)
}

fn unseal(key: &LessSafeKey, account: &str, blob: &[u8]) -> Option<Secret> {
    let (nonce, sealed) = blob.split_at_checked(NONCE_LEN)?;
    // `open_in_place` decrypts into this buffer, so the scratch copy *becomes*
    // the plaintext and has to be wiped like one. Held as a `Secret` from the
    // moment it is allocated rather than after it holds anything, so there is
    // no early return between the decryption and the thing that wipes it.
    let mut scratch = Secret::new(sealed.to_vec());
    let opened = key
        .open_in_place(
            Nonce::try_assume_unique_for_key(nonce).ok()?,
            Aad::from(account.as_bytes()),
            scratch.expose_mut(),
        )
        .ok()?
        .to_vec();
    Some(Secret::new(opened))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One directory per test, because these write files and the suite runs in
    /// parallel. Named for the test so a leftover says which one left it.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ducktape-vault-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn secret(bytes: &[u8]) -> Secret {
        Secret::new(bytes.to_vec())
    }

    fn opened(held: Held) -> Vec<u8> {
        match held {
            Held::Secret(secret) => secret.expose().to_vec(),
            other => panic!("expected the secret, got {other:?}"),
        }
    }

    #[test]
    fn a_sealed_secret_comes_back_under_the_passphrase_that_sealed_it() {
        let dir = scratch("round-trip");
        let key = [7_u8; 32];
        keep_in(&dir, "wallet:0xabc", &secret(&key), "correct horse").expect("sealed");
        assert_eq!(
            opened(take_in(&dir, "wallet:0xabc", "correct horse").expect("read back")),
            key,
        );
    }

    #[test]
    fn nothing_readable_is_written_to_the_file() {
        let dir = scratch("ciphertext");
        let key = [0xAB_u8; 32];
        keep_in(&dir, "wallet:0xabc", &secret(&key), "pass").expect("sealed");
        let raw = std::fs::read(path(&dir)).expect("the file");
        // The whole point of the module. A window over the plaintext anywhere
        // in the file — including one the base64 happens to align with — is the
        // failure this exists to prevent.
        assert!(
            !raw.windows(key.len()).any(|window| window == key),
            "the file holds the secret in the clear"
        );
        let text = String::from_utf8(raw).expect("json is text");
        assert!(text.contains(SCHEME), "the file says which scheme it is");
        assert!(!text.contains(&BASE64.encode(key)), "nor base64 of it");
    }

    /// The blob is ciphertext, so this is not what protects it — but a file at
    /// 0644 is every other account on this machine holding a copy to work on
    /// offline, and the directory mode is what stops them listing which
    /// accounts this machine even has.
    #[cfg(unix)]
    #[test]
    fn the_key_file_and_its_directory_are_readable_by_nobody_else() {
        use std::os::unix::fs::PermissionsExt;

        let dir = scratch("permissions");
        keep_in(&dir, "wallet:0xabc", &secret(&[1; 32]), "pass").expect("sealed");
        let mode = |at: &Path| {
            std::fs::metadata(at)
                .expect("it is there")
                .permissions()
                .mode()
                & 0o777
        };
        assert_eq!(mode(&path(&dir)), 0o600);
        assert_eq!(mode(&dir), 0o700);

        // And a second write, which goes through the staging file again.
        keep_in(&dir, "hyperliquid-mainnet:0xabc", &secret(&[2; 32]), "pass").expect("sealed");
        assert_eq!(mode(&path(&dir)), 0o600);
        // With nothing left beside it.
        assert!(!path(&dir).with_extension("json.new").exists());
    }

    #[test]
    fn a_wrong_passphrase_is_a_refusal_rather_than_a_fault_or_a_cancelled_sheet() {
        let dir = scratch("wrong-passphrase");
        keep_in(&dir, "wallet:0xabc", &secret(&[1; 32]), "right").expect("sealed");
        let refused = take_in(&dir, "wallet:0xabc", "wrong").expect_err("refused");
        // `Refused` rather than `Platform`, because the owner mistyping is a
        // thing to have another go at — and rather than `Held::Declined`,
        // whose sentence in `read_key` is about a Touch ID sheet this build
        // never raised.
        assert_eq!(refused.cause, crate::session::Cause::Refused);
        // And it says which file, because a lost passphrase has one way out.
        assert!(refused.message.contains("keys.json"));
    }

    #[test]
    fn an_item_cannot_be_read_under_another_accounts_name() {
        let dir = scratch("aad");
        keep_in(&dir, "wallet:0xaaa", &secret(&[2; 32]), "pass").expect("sealed");
        // Moved by hand, which is what an attacker with the file can do. The
        // account name is the associated data, so the tag stops opening.
        let mut file = read(&dir).expect("the file");
        let blob = file.items.remove("wallet:0xaaa").expect("the item");
        file.items.insert("wallet:0xbbb".to_owned(), blob);
        write(&dir, &file).expect("rewritten");
        // Refused, and under the same sentence a wrong passphrase gets: the
        // tag does not verify and nothing here can tell which of the two it
        // was. The advice is the same either way, which is why one sentence is
        // honest for both.
        assert_eq!(
            take_in(&dir, "wallet:0xbbb", "pass")
                .expect_err("refused")
                .cause,
            crate::session::Cause::Refused,
        );
    }

    #[test]
    fn two_items_share_one_file_and_one_passphrase() {
        let dir = scratch("two-items");
        keep_in(&dir, "wallet:0xabc", &secret(&[3; 32]), "pass").expect("wallet");
        keep_in(&dir, "hyperliquid-mainnet:0xabc", &secret(&[4; 32]), "pass").expect("agent");
        assert_eq!(
            opened(take_in(&dir, "wallet:0xabc", "pass").expect("wallet back")),
            [3; 32],
        );
        assert_eq!(
            opened(take_in(&dir, "hyperliquid-mainnet:0xabc", "pass").expect("agent back")),
            [4; 32],
        );
        assert!(holds_in(&dir, "wallet:0xabc"));
        assert!(occupied_in(&dir));
    }

    #[test]
    fn a_second_passphrase_is_refused_rather_than_stranding_what_is_there() {
        let dir = scratch("one-passphrase");
        keep_in(&dir, "wallet:0xabc", &secret(&[5; 32]), "first").expect("sealed");
        // Writing under a different passphrase would leave one file holding
        // items from two, half of which the owner can no longer open and
        // nothing on screen saying which half.
        let refused = keep_in(
            &dir,
            "hyperliquid-mainnet:0xabc",
            &secret(&[6; 32]),
            "second",
        )
        .expect_err("a second passphrase is refused");
        assert_eq!(refused.cause, crate::session::Cause::Refused);
        // And the first item is exactly where it was.
        assert_eq!(
            opened(take_in(&dir, "wallet:0xabc", "first").expect("still there")),
            [5; 32],
        );
        assert!(!holds_in(&dir, "hyperliquid-mainnet:0xabc"));
    }

    #[test]
    fn replacing_an_item_keeps_the_others() {
        let dir = scratch("replace");
        keep_in(&dir, "wallet:0xabc", &secret(&[7; 32]), "pass").expect("wallet");
        keep_in(&dir, "lighter-mainnet:0xabc", &secret(&[8; 40]), "pass").expect("agent");
        keep_in(&dir, "lighter-mainnet:0xabc", &secret(&[9; 40]), "pass").expect("re-enrolled");
        assert_eq!(
            opened(take_in(&dir, "lighter-mainnet:0xabc", "pass").expect("the new one")),
            [9; 40],
        );
        assert_eq!(
            opened(take_in(&dir, "wallet:0xabc", "pass").expect("the untouched one")),
            [7; 32],
        );
    }

    #[test]
    fn an_empty_passphrase_asks_rather_than_seals() {
        let dir = scratch("empty-passphrase");
        let asked = keep_in(&dir, "wallet:0xabc", &secret(&[1; 32]), "").expect_err("asked");
        assert_eq!(asked.cause, crate::session::Cause::WantsPassphrase);
        assert!(!path(&dir).exists(), "and nothing was written");

        keep_in(&dir, "wallet:0xabc", &secret(&[1; 32]), "pass").expect("sealed");
        let asked = take_in(&dir, "wallet:0xabc", "").expect_err("asked on the way back too");
        assert_eq!(asked.cause, crate::session::Cause::WantsPassphrase);
    }

    #[test]
    fn an_account_with_nothing_in_the_file_is_missing_rather_than_refused() {
        let dir = scratch("missing");
        // No file at all.
        assert!(matches!(
            take_in(&dir, "wallet:0xabc", "pass").expect("an answer"),
            Held::Missing
        ));
        assert!(!holds_in(&dir, "wallet:0xabc"));
        assert!(!occupied_in(&dir));
        // A file, but not this account — and asked before the passphrase is,
        // so a reader is not made to type one to be told there is nothing here.
        keep_in(&dir, "wallet:0xabc", &secret(&[1; 32]), "pass").expect("sealed");
        assert!(matches!(
            take_in(&dir, "wallet:0xdef", "").expect("an answer"),
            Held::Missing
        ));
    }

    #[test]
    fn every_write_seals_under_a_nonce_of_its_own() {
        let dir = scratch("nonce");
        keep_in(&dir, "wallet:0xabc", &secret(&[1; 32]), "pass").expect("once");
        let first = read(&dir).expect("the file").items["wallet:0xabc"].clone();
        keep_in(&dir, "wallet:0xabc", &secret(&[1; 32]), "pass").expect("twice");
        let second = read(&dir).expect("the file").items["wallet:0xabc"].clone();
        // The same secret under the same passphrase, and the bytes on disk
        // differ — which is the nonce doing its job, and is what stops a reader
        // of the file from telling a re-enrolment that changed nothing from one
        // that changed a key.
        assert_ne!(first, second);
    }

    #[test]
    fn a_file_this_app_did_not_write_is_refused_rather_than_guessed_at() {
        let dir = scratch("foreign");
        std::fs::create_dir_all(&dir).expect("the directory");
        std::fs::write(path(&dir), r#"{"scheme":"something-else","items":{}}"#).expect("written");
        assert!(read(&dir).is_err());
        // Reported, not read as empty. `Missing` here would tell a reader whose
        // keys are in that file to enrol again, and the store below would then
        // write a fresh one over the top of them.
        assert!(take_in(&dir, "wallet:0xabc", "pass").is_err());
    }

    /// The data-loss arm, and the reason the two reads above distinguish "no
    /// file" from "a file I could not read".
    #[test]
    fn a_file_that_will_not_parse_is_never_replaced_by_a_fresh_one() {
        let dir = scratch("no-clobber");
        std::fs::create_dir_all(&dir).expect("the directory");
        let foreign = r#"{"scheme":"ducktape-passphrase-1","salt":"!!not base64"}"#;
        std::fs::write(path(&dir), foreign).expect("written");
        assert!(keep_in(&dir, "wallet:0xabc", &secret(&[1; 32]), "pass").is_err());
        // Byte for byte. A store that could not read the file must not have
        // touched it.
        assert_eq!(
            std::fs::read_to_string(path(&dir)).expect("still there"),
            foreign,
        );
    }
}
