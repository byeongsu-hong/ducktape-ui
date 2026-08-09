//! The signature `session.rs` assumes and never produces.
//!
//! `session.rs` models who may sign; this file is the signing. The app holds an
//! *agent* key — a keypair the master wallet approved once, which can place and
//! cancel orders and can never withdraw — so the worst a leak of anything here
//! can do is trade badly, not empty an account.
//!
//! Hyperliquid signs with EIP-712 typed data, and it uses **two different
//! schemes** depending on the action. Both are implemented below because the
//! three actions this app needs straddle them:
//!
//! * **L1 actions** (`order`, `cancel`) hash the action itself: MessagePack the
//!   action, append the nonce and a vault byte, keccak it, and sign that hash as
//!   the `connectionId` of a phantom `Agent` struct under a fixed `Exchange`
//!   domain on chain 1337. The action's *field order* is inside the hash, so
//!   the payload is not just JSON — it is a byte string that must be rebuilt
//!   exactly.
//! * **User-signed actions** (`approveAgent`) are ordinary EIP-712: the action's
//!   own fields are the message, under a `HyperliquidSignTransaction` domain
//!   whose chain id the action carries in `signatureChainId`.
//!
//! Because an L1 signature covers MessagePack bytes while the request body is
//! JSON, the two encodings must describe the same action or the exchange
//! recovers a stranger. [`Val`] is that single source: one ordered tree, packed
//! for the hash and converted for the body, so a field can never be signed in
//! one shape and sent in another.
//!
//! **Nothing here submits anything.** This module builds and signs; the only
//! code that posts to `/exchange` lives in the ignored tests below, and it posts
//! with keys that control nothing.
//!
//! What `api.hyperliquid.xyz` itself confirmed while this was written, each
//! re-checked by an ignored test:
//!
//! - `approveAgent` needs exactly `type`, `hyperliquidChain`, `signatureChainId`,
//!   `agentAddress` and `nonce`. `agentName` is optional on the wire but is in
//!   the *signed* type list, and a signature made over `""` with the field left
//!   out still recovers — so an absent name hashes as the empty string.
//!   A stale nonce is refused before the signature is even read
//!   (`Invalid nonce: nonce too low 1754600000000 < 1786024550598`), so a nonce
//!   is a clock reading, not a counter.
//! - Every rejection past deserialization **names the address it recovered**.
//!   That is what makes the payload verifiable without executing anything: sign
//!   with a fresh key, and if the exchange echoes that key's own address, its
//!   hashing and ours agree byte for byte. A wrong domain, type string, field
//!   order or MessagePack byte would echo a stranger.
//! - An unauthorised signer is told `Must deposit before performing actions.
//!   User: 0x…` for `approveAgent`, and `User or API Wallet 0x… does not exist.`
//!   for `order` and `cancel` — authorisation complaints, on the far side of
//!   deserialization, with nothing executed.

#![allow(dead_code)]

use std::fmt;

use k256::ecdsa::{RecoveryId, Signature as Ecdsa, SigningKey, VerifyingKey};
use serde_json::{Value, json};
use sha3::{Digest, Keccak256};

use crate::hyperliquid::HlError;

/// The `Exchange` domain's chain id. Not a real chain: L1 actions are signed
/// off-chain, so the domain only has to be a constant both sides agree on.
const L1_CHAIN_ID: u64 = 1337;

/// The chain a wallet claims to be signing on for user-signed actions. Any
/// chain works — it is the wallet's own display and replay context, while
/// `hyperliquidChain` is what pins the action to mainnet or testnet — so this
/// matches the official SDKs rather than inventing a value.
const SIGNATURE_CHAIN_ID: u64 = 0x66eee;

/// Both domains share these; only the name and chain id differ.
const DOMAIN_TYPE: &str =
    "EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)";

fn err(message: impl Into<String>) -> HlError {
    HlError {
        message: message.into(),
    }
}

fn keccak(bytes: &[u8]) -> [u8; 32] {
    Keccak256::digest(bytes).into()
}

// ---------------------------------------------------------------------------
// Addresses and keys
// ---------------------------------------------------------------------------

/// A 20-byte Ethereum address, kept as bytes because EIP-712 needs it as bytes
/// and the wire needs it as text; a `String` would have to be re-parsed at the
/// one place where getting it wrong is unrecoverable.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Address([u8; 20]);

impl Address {
    /// Strict on purpose: this is a trust boundary. An address that arrived
    /// truncated, or as a bare hex string some other tool trimmed, must not
    /// quietly become a *different* valid address once it is zero-padded into
    /// an EIP-712 word.
    pub fn parse(text: &str) -> Result<Self, HlError> {
        let body = text
            .strip_prefix("0x")
            .ok_or_else(|| err(format!("address must start with 0x: {text}")))?;
        let bytes = hex::decode(body).map_err(|error| err(format!("bad address: {error}")))?;
        let bytes: [u8; 20] = bytes
            .try_into()
            .map_err(|_| err(format!("an address is 20 bytes: {text}")))?;
        Ok(Self(bytes))
    }

    pub fn bytes(&self) -> &[u8; 20] {
        &self.0
    }
}

impl fmt::Display for Address {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "0x{}", hex::encode(self.0))
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self}")
    }
}

/// An agent keypair.
///
/// No `Clone`, no `Display`, and a hand-written `Debug` that prints the public
/// address only: the scalar has exactly one legitimate destination, which is the
/// signing call below. `k256`'s `SigningKey` zeroizes itself on drop, so this
/// type does not need to.
pub struct Wallet {
    key: SigningKey,
    address: Address,
}

impl Wallet {
    /// A fresh agent key from the OS entropy pool.
    ///
    /// The retry is not superstition: `from_slice` rejects zero and anything at
    /// or above the curve order, and a key generator that returns "sometimes"
    /// is worse than one that loops on odds of about 2^-128.
    pub fn generate() -> Self {
        loop {
            let mut bytes = [0u8; 32];
            getrandom::fill(&mut bytes).expect("no OS entropy to make a key from");
            if let Ok(wallet) = Self::from_secret(&bytes) {
                return wallet;
            }
        }
    }

    pub fn from_secret(bytes: &[u8; 32]) -> Result<Self, HlError> {
        let key = SigningKey::from_slice(bytes)
            .map_err(|error| err(format!("not a usable secp256k1 key: {error}")))?;
        let address = address_of(key.verifying_key());
        Ok(Self { key, address })
    }

    pub fn address(&self) -> Address {
        self.address
    }

    /// RFC 6979 deterministic ECDSA with the low-`s` normalisation Ethereum
    /// requires, both of which `k256` does here; the recovery id it hands back
    /// is what turns an (r, s) into an address the exchange can recover.
    pub fn sign(&self, action: &Action) -> Signature {
        self.sign_digest(action.digest())
    }

    fn sign_digest(&self, digest: [u8; 32]) -> Signature {
        let (signature, recovery) = self
            .key
            .sign_prehash_recoverable(&digest)
            .expect("a 32-byte digest is always signable");
        let bytes = signature.to_bytes();
        let mut r = [0u8; 32];
        let mut s = [0u8; 32];
        r.copy_from_slice(&bytes[..32]);
        s.copy_from_slice(&bytes[32..]);
        Signature {
            r,
            s,
            // Ethereum's v is the recovery id offset by 27, a leftover of the
            // pre-EIP-155 transaction format that every wallet still speaks.
            v: 27 + recovery.to_byte(),
        }
    }
}

impl fmt::Debug for Wallet {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Wallet({})", self.address)
    }
}

/// The address is the last 20 bytes of the keccak of the 64 coordinate bytes —
/// the `0x04` uncompressed-form tag is dropped, and hashing it instead is the
/// classic way to derive an address nobody else agrees with.
fn address_of(key: &VerifyingKey) -> Address {
    let point = key.to_encoded_point(false);
    let hash = keccak(&point.as_bytes()[1..]);
    let mut bytes = [0u8; 20];
    bytes.copy_from_slice(&hash[12..]);
    Address(bytes)
}

/// What the exchange wants beside the action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Signature {
    pub r: [u8; 32],
    pub s: [u8; 32],
    pub v: u8,
}

impl Signature {
    /// Recovering our own signer and comparing it to the key we signed with is
    /// the only offline check that the digest we signed is the digest we think
    /// it is — and it is what the exchange does to us.
    pub fn recover(&self, digest: [u8; 32]) -> Result<Address, HlError> {
        let recovery = RecoveryId::from_byte(self.v.wrapping_sub(27))
            .ok_or_else(|| err(format!("v must be 27 or 28, got {}", self.v)))?;
        let mut raw = [0u8; 64];
        raw[..32].copy_from_slice(&self.r);
        raw[32..].copy_from_slice(&self.s);
        let signature =
            Ecdsa::from_slice(&raw).map_err(|error| err(format!("bad signature: {error}")))?;
        let key = VerifyingKey::recover_from_prehash(&digest, &signature, recovery)
            .map_err(|error| err(format!("no signer recovers from this: {error}")))?;
        Ok(address_of(&key))
    }

    pub fn json(&self) -> Value {
        json!({
            "r": format!("0x{}", hex::encode(self.r)),
            "s": format!("0x{}", hex::encode(self.s)),
            "v": self.v,
        })
    }
}

// ---------------------------------------------------------------------------
// EIP-712
// ---------------------------------------------------------------------------

/// Every EIP-712 field is one 32-byte word: an integer is big-endian and
/// left-padded, an address is left-padded to the same width, and a string is
/// replaced by the keccak of its bytes.
fn word_u64(value: u64) -> [u8; 32] {
    let mut word = [0u8; 32];
    word[24..].copy_from_slice(&value.to_be_bytes());
    word
}

fn word_address(address: &Address) -> [u8; 32] {
    let mut word = [0u8; 32];
    word[12..].copy_from_slice(address.bytes());
    word
}

fn hash_struct(type_string: &str, fields: &[[u8; 32]]) -> [u8; 32] {
    let mut buffer = Vec::with_capacity(32 * (fields.len() + 1));
    buffer.extend_from_slice(&keccak(type_string.as_bytes()));
    for field in fields {
        buffer.extend_from_slice(field);
    }
    keccak(&buffer)
}

/// A domain separator is just a struct hash of the domain, and both of
/// Hyperliquid's domains use version "1" and the zero verifying contract.
fn domain_separator(name: &str, chain_id: u64) -> [u8; 32] {
    hash_struct(
        DOMAIN_TYPE,
        &[
            keccak(name.as_bytes()),
            keccak(b"1"),
            word_u64(chain_id),
            [0u8; 32],
        ],
    )
}

/// `0x19 0x01` is EIP-191's "this is structured data, not a transaction" tag;
/// without it a signature over a payload could be replayed as a transaction.
fn signing_hash(domain: [u8; 32], message: [u8; 32]) -> [u8; 32] {
    let mut buffer = [0u8; 66];
    buffer[..2].copy_from_slice(&[0x19, 0x01]);
    buffer[2..34].copy_from_slice(&domain);
    buffer[34..].copy_from_slice(&message);
    keccak(&buffer)
}

// ---------------------------------------------------------------------------
// The one action encoding, in both shapes
// ---------------------------------------------------------------------------

/// An action, ordered.
///
/// Order is load-bearing: it is inside the L1 hash. That rules out
/// `serde_json::Value` as the source of truth, whose object is a sorted map, so
/// this tree exists to be MessagePacked for the signature *and* converted for
/// the body from the same values.
#[derive(Clone, Debug, PartialEq)]
enum Val {
    Str(String),
    Bool(bool),
    Uint(u64),
    Arr(Vec<Val>),
    Map(Vec<(&'static str, Val)>),
}

impl Val {
    fn pack(&self, out: &mut Vec<u8>) {
        match self {
            Val::Str(text) => pack_str(out, text),
            Val::Bool(flag) => out.push(if *flag { 0xc3 } else { 0xc2 }),
            Val::Uint(value) => pack_uint(out, *value),
            Val::Arr(items) => {
                pack_len(out, items.len(), 0x90, 0xdc, 0xdd);
                for item in items {
                    item.pack(out);
                }
            }
            Val::Map(pairs) => {
                pack_len(out, pairs.len(), 0x80, 0xde, 0xdf);
                for (key, value) in pairs {
                    pack_str(out, key);
                    value.pack(out);
                }
            }
        }
    }

    fn json(&self) -> Value {
        match self {
            Val::Str(text) => Value::from(text.as_str()),
            Val::Bool(flag) => Value::from(*flag),
            Val::Uint(value) => Value::from(*value),
            Val::Arr(items) => Value::from(items.iter().map(Val::json).collect::<Vec<_>>()),
            Val::Map(pairs) => Value::Object(
                pairs
                    .iter()
                    .map(|(key, value)| ((*key).to_owned(), value.json()))
                    .collect(),
            ),
        }
    }
}

/// MessagePack, only as much of it as an action can contain. Written out rather
/// than pulled in because a serde MessagePack crate would still need a
/// field-ordered value type to feed it, and that type is the actual work.
fn pack_len(out: &mut Vec<u8>, len: usize, fix: u8, wide16: u8, wide32: u8) {
    match u16::try_from(len) {
        Ok(_) if len < 16 => out.push(fix | len as u8),
        Ok(short) => {
            out.push(wide16);
            out.extend_from_slice(&short.to_be_bytes());
        }
        Err(_) => {
            out.push(wide32);
            out.extend_from_slice(&(len as u32).to_be_bytes());
        }
    }
}

fn pack_str(out: &mut Vec<u8>, text: &str) {
    let bytes = text.as_bytes();
    match bytes.len() {
        len if len < 32 => out.push(0xa0 | len as u8),
        len if len <= u8::MAX as usize => out.extend_from_slice(&[0xd9, len as u8]),
        len if len <= u16::MAX as usize => {
            out.push(0xda);
            out.extend_from_slice(&(len as u16).to_be_bytes());
        }
        len => {
            out.push(0xdb);
            out.extend_from_slice(&(len as u32).to_be_bytes());
        }
    }
    out.extend_from_slice(bytes);
}

fn pack_uint(out: &mut Vec<u8>, value: u64) {
    match value {
        small if small < 0x80 => out.push(small as u8),
        value if value <= u8::MAX as u64 => out.extend_from_slice(&[0xcc, value as u8]),
        value if value <= u16::MAX as u64 => {
            out.push(0xcd);
            out.extend_from_slice(&(value as u16).to_be_bytes());
        }
        value if value <= u32::MAX as u64 => {
            out.push(0xce);
            out.extend_from_slice(&(value as u32).to_be_bytes());
        }
        value => {
            out.push(0xcf);
            out.extend_from_slice(&value.to_be_bytes());
        }
    }
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

/// Which deployment an action is for. This is signed over twice — as the
/// `hyperliquidChain` string in a user-signed action, and as the phantom
/// agent's one-letter `source` in an L1 one — so a mainnet signature cannot be
/// replayed on testnet or the other way round.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chain {
    Mainnet,
    Testnet,
}

impl Chain {
    fn name(self) -> &'static str {
        match self {
            Chain::Mainnet => "Mainnet",
            Chain::Testnet => "Testnet",
        }
    }

    fn source(self) -> &'static str {
        match self {
            Chain::Mainnet => "a",
            Chain::Testnet => "b",
        }
    }

    /// Where a signed action is posted.
    pub fn exchange_url(self) -> &'static str {
        match self {
            Chain::Mainnet => "https://api.hyperliquid.xyz/exchange",
            Chain::Testnet => "https://api.hyperliquid-testnet.xyz/exchange",
        }
    }

    /// Where every read is asked, which is the same deployment the writes go
    /// to and must never be a separate setting.
    ///
    /// The two endpoints live on one value because the alternative has a
    /// failure with no symptom: a screen reading mainnet prices while a
    /// signature carries `Testnet` prices an order against a book it will
    /// never reach, and both halves look right on their own. `Chain` is
    /// already the thing a signature is pinned to — it is the phantom agent's
    /// `source` and the user-signed `hyperliquidChain`, so a mainnet signature
    /// cannot be replayed on testnet — and making it the thing a *read* is
    /// pinned to as well is what stops the screen and the order disagreeing
    /// about which deployment is being traded.
    pub fn info_url(self) -> &'static str {
        match self {
            Chain::Mainnet => "https://api.hyperliquid.xyz/info",
            Chain::Testnet => "https://api.hyperliquid-testnet.xyz/info",
        }
    }

    pub fn ws_url(self) -> &'static str {
        match self {
            Chain::Mainnet => "wss://api.hyperliquid.xyz/ws",
            Chain::Testnet => "wss://api.hyperliquid-testnet.xyz/ws",
        }
    }

    /// A short, stable name for this deployment, for anything that has to file
    /// something under it. Not for a reader — `venue.rs` names networks — and
    /// deliberately not the wire spelling, so changing what a keychain item is
    /// called can never change what a signature says.
    pub fn key(self) -> &'static str {
        match self {
            Chain::Mainnet => "mainnet",
            Chain::Testnet => "testnet",
        }
    }

    /// Whether this deployment trades money that is worth something.
    ///
    /// Read by the screen rather than by the signer: what it changes is how
    /// loudly the app says which network an order is about to go to, and that
    /// is the one label a trader may never have to work out for themselves.
    pub fn testnet(self) -> bool {
        matches!(self, Chain::Testnet)
    }
}

/// How an action's digest is reached.
#[derive(Clone, Debug, PartialEq)]
enum Scheme {
    /// MessagePack the action and sign the hash of it.
    L1,
    /// Sign the action's own fields as EIP-712. The struct hash is built by the
    /// builder, where each field's EIP-712 type is still known; recovering that
    /// from [`Val`] would mean carrying a second type per field for one action.
    User { struct_hash: [u8; 32] },
}

/// A built, unsigned action: the body that goes on the wire, the nonce that is
/// signed with it, and the chain both are pinned to. Everything a signature
/// depends on is in here, so [`Action::digest`] takes no arguments and cannot
/// be handed a chain or nonce that disagrees with the body.
#[derive(Clone, Debug, PartialEq)]
pub struct Action {
    chain: Chain,
    nonce: u64,
    body: Val,
    scheme: Scheme,
}

impl Action {
    pub fn chain(&self) -> Chain {
        self.chain
    }

    pub fn nonce(&self) -> u64 {
        self.nonce
    }

    /// The 32 bytes a wallet actually signs.
    pub fn digest(&self) -> [u8; 32] {
        match self.scheme {
            Scheme::User { struct_hash } => signing_hash(
                domain_separator("HyperliquidSignTransaction", SIGNATURE_CHAIN_ID),
                struct_hash,
            ),
            Scheme::L1 => {
                let agent = hash_struct(
                    "Agent(string source,bytes32 connectionId)",
                    &[keccak(self.chain.source().as_bytes()), self.connection_id()],
                );
                signing_hash(domain_separator("Exchange", L1_CHAIN_ID), agent)
            }
        }
    }

    /// The L1 action hash: MessagePack, then the nonce, then a byte saying
    /// whether a vault address follows. This app trades its own account, so the
    /// byte is always "no vault"; a vault would append its 20 bytes here.
    fn connection_id(&self) -> [u8; 32] {
        let mut packed = Vec::new();
        self.body.pack(&mut packed);
        packed.extend_from_slice(&self.nonce.to_be_bytes());
        packed.push(0x00);
        keccak(&packed)
    }

    /// The complete POST body for `/exchange`. Nothing in this module sends it.
    pub fn request(&self, wallet: &Wallet) -> Value {
        json!({
            "action": self.body.json(),
            "nonce": self.nonce,
            "signature": wallet.sign(self).json(),
        })
    }
}

/// Approve an agent wallet. Signed by the **master** wallet, not by the agent:
/// this is the one action in this file the app cannot sign for itself, which is
/// the entire point of an agent key.
///
/// There is no expiry argument because the action has no field for one — the
/// exchange picks the window and reports it back through `extraAgents`.
pub fn approve_agent(chain: Chain, agent: Address, name: &str, nonce_ms: u64) -> Action {
    let body = Val::Map(vec![
        ("type", Val::Str("approveAgent".into())),
        ("hyperliquidChain", Val::Str(chain.name().into())),
        (
            "signatureChainId",
            Val::Str(format!("0x{SIGNATURE_CHAIN_ID:x}")),
        ),
        ("agentAddress", Val::Str(agent.to_string())),
        ("agentName", Val::Str(name.to_owned())),
        ("nonce", Val::Uint(nonce_ms)),
    ]);
    // `signatureChainId` is deliberately absent from the signed fields: it names
    // the domain rather than sitting in it, so the exchange reads it from the
    // body to rebuild the domain it verifies against.
    let struct_hash = hash_struct(
        "HyperliquidTransaction:ApproveAgent(string hyperliquidChain,address agentAddress,string agentName,uint64 nonce)",
        &[
            keccak(chain.name().as_bytes()),
            word_address(&agent),
            keccak(name.as_bytes()),
            word_u64(nonce_ms),
        ],
    );
    Action {
        chain,
        nonce: nonce_ms,
        body,
        scheme: Scheme::User { struct_hash },
    }
}

/// How long an order stays alive. `Gtc` rests, `Ioc` takes what is there and
/// cancels the rest, `Alo` refuses to cross and so can only ever be a maker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tif {
    Alo,
    Ioc,
    Gtc,
}

impl Tif {
    fn name(self) -> &'static str {
        match self {
            Tif::Alo => "Alo",
            Tif::Ioc => "Ioc",
            Tif::Gtc => "Gtc",
        }
    }
}

/// One limit order. `asset` is the market's index in `meta.universe`, not its
/// name — the wire carries the index, and the name never reaches the exchange.
///
/// Trigger (stop/take-profit) orders use a different `t` payload and are not
/// built here; nothing in this app places one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Order {
    pub asset: u32,
    pub buy: bool,
    pub price: f64,
    pub size: f64,
    pub reduce_only: bool,
    pub tif: Tif,
}

pub fn order(chain: Chain, orders: &[Order], nonce_ms: u64) -> Result<Action, HlError> {
    let wires = orders
        .iter()
        .map(|order| {
            Ok(Val::Map(vec![
                ("a", Val::Uint(order.asset.into())),
                ("b", Val::Bool(order.buy)),
                ("p", Val::Str(wire_num(order.price)?)),
                ("s", Val::Str(wire_num(order.size)?)),
                ("r", Val::Bool(order.reduce_only)),
                (
                    "t",
                    Val::Map(vec![(
                        "limit",
                        Val::Map(vec![("tif", Val::Str(order.tif.name().into()))]),
                    )]),
                ),
            ]))
        })
        .collect::<Result<Vec<_>, HlError>>()?;
    Ok(Action {
        chain,
        nonce: nonce_ms,
        body: Val::Map(vec![
            ("type", Val::Str("order".into())),
            ("orders", Val::Arr(wires)),
            // Only relevant to bracket orders, and required regardless.
            ("grouping", Val::Str("na".into())),
        ]),
        scheme: Scheme::L1,
    })
}

/// One resting order to pull, by the id the exchange gave it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cancel {
    pub asset: u32,
    pub oid: u64,
}

pub fn cancel(chain: Chain, cancels: &[Cancel], nonce_ms: u64) -> Action {
    let wires = cancels
        .iter()
        .map(|cancel| {
            Val::Map(vec![
                ("a", Val::Uint(cancel.asset.into())),
                ("o", Val::Uint(cancel.oid)),
            ])
        })
        .collect();
    Action {
        chain,
        nonce: nonce_ms,
        body: Val::Map(vec![
            ("type", Val::Str("cancel".into())),
            ("cancels", Val::Arr(wires)),
        ]),
        scheme: Scheme::L1,
    }
}

/// Prices and sizes travel as decimal strings, and the *string* is what gets
/// hashed — so a float has to become the one spelling the exchange expects:
/// eight decimal places at most, no trailing zeros, no `-0`.
///
/// The rounding is checked rather than accepted. Silently shipping 0.00001231
/// as 0.00001 is the kind of bug that only shows up as a fill you did not want.
///
/// ponytail: this does not enforce Hyperliquid's tick and lot rules (five
/// significant figures, per-market size decimals) — the exchange rejects those,
/// and doing it here would mean carrying `meta.universe` into a signing module.
fn wire_num(value: f64) -> Result<String, HlError> {
    if !value.is_finite() {
        return Err(err(format!("{value} is not a number the wire can carry")));
    }
    let rendered = format!("{value:.8}");
    let round_trip: f64 = rendered
        .parse()
        .expect("a fixed-point format always parses");
    if (round_trip - value).abs() >= 1e-12 {
        return Err(err(format!(
            "{value} needs more than the 8 decimals the wire allows"
        )));
    }
    let trimmed = rendered.trim_end_matches('0').trim_end_matches('.');
    Ok(if trimmed == "-0" { "0" } else { trimmed }.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// The key every Hyperliquid SDK's signing test uses, so the vectors below
    /// are theirs rather than ours.
    const VECTOR_KEY: &str = "0123456789012345678901234567890123456789012345678901234567890123";

    fn vector_wallet() -> Wallet {
        let mut bytes = [0u8; 32];
        hex::decode_to_slice(VECTOR_KEY, &mut bytes).expect("the vector key is 32 bytes of hex");
        Wallet::from_secret(&bytes).expect("the vector key is a valid secp256k1 key")
    }

    /// The published vectors print r and s as minimal hex, dropping leading
    /// zeros; the wire wants a full 32-byte word. Padding the *vector* keeps the
    /// quoted constants exactly as their source wrote them.
    fn padded(vector: &str) -> String {
        format!(
            "0x{:0>64}",
            vector.strip_prefix("0x").expect("vectors are 0x-prefixed")
        )
    }

    fn hex32(word: [u8; 32]) -> String {
        format!("0x{}", hex::encode(word))
    }

    fn signed(wallet: &Wallet, action: &Action) -> (String, String, u8) {
        let signature = wallet.sign(action);
        (hex32(signature.r), hex32(signature.s), signature.v)
    }

    fn eth_order(asset: u32, price: f64, size: f64, tif: Tif) -> Order {
        Order {
            asset,
            buy: true,
            price,
            size,
            reduce_only: false,
            tif,
        }
    }

    /// `cast wallet address` (Foundry) derives this from the same key. An
    /// address is keccak of the *public* key, so nothing else in this file
    /// proves the derivation: the signature vectors below match whether or not
    /// the address is right.
    #[test]
    fn the_address_is_the_one_an_independent_wallet_derives() {
        assert_eq!(
            vector_wallet().address().to_string(),
            "0x14791697260e4c9a71f18484c9f997b308e59325"
        );
    }

    /// The MessagePack bytes and the action hash, pinned by the connection id
    /// the Hyperliquid Python SDK asserts against production. This is the check
    /// that a field order, a map header or a decimal spelling has not drifted —
    /// none of which the exchange could tell us apart from a bad key.
    #[test]
    fn the_action_hash_matches_the_exchanges_own_vector() {
        let action = order(
            Chain::Mainnet,
            &[eth_order(4, 1670.1, 0.0147, Tif::Ioc)],
            1_677_777_606_040,
        )
        .expect("the vector's price and size fit the wire");
        assert_eq!(
            hex32(action.connection_id()),
            "0x0fcbeda5ae3c4950a548021552a4fea2226858c4453571bf3f24ba017eac2908"
        );
    }

    /// The rest of the L1 path — phantom agent, `Exchange` domain, the `0x1901`
    /// prefix, RFC 6979, low-`s`, and v's offset — against the same SDK's
    /// vectors. Both chains, because `source` is the only difference between
    /// them and a signature that ignored it would still look fine on one.
    #[test]
    fn an_l1_signature_matches_the_sdk_vectors() {
        let wallet = vector_wallet();
        let mainnet = order(Chain::Mainnet, &[eth_order(1, 100.0, 100.0, Tif::Gtc)], 0)
            .expect("a round price and size fit the wire");
        assert_eq!(
            signed(&wallet, &mainnet),
            (
                padded("0xd65369825a9df5d80099e513cce430311d7d26ddf477f5b3a33d2806b100d78e"),
                padded("0x2b54116ff64054968aa237c20ca9ff68000f977c93289157748a3162b6ea940e"),
                28,
            )
        );

        let testnet = order(Chain::Testnet, &[eth_order(1, 100.0, 100.0, Tif::Gtc)], 0)
            .expect("a round price and size fit the wire");
        assert_eq!(
            signed(&wallet, &testnet),
            (
                padded("0x82b2ba28e76b3d761093aaded1b1cdad4960b3af30212b343fb2e6cdfa4e3d54"),
                padded("0x6b53878fc99d26047f4d7e8c90eb98955a109f44209163f52d8dc4278cbbd9f5"),
                27,
            )
        );
    }

    /// The user-signed path has no published SDK vector, so this one comes from
    /// Foundry: `cast wallet sign --data` over the EIP-712 document these field
    /// names and types describe. A different implementation of the same spec,
    /// which is the point — matching our own encoder would prove nothing.
    #[test]
    fn an_approve_agent_signature_matches_foundry() {
        let agent = Address::parse("0x5e9ee1089755c3435139848e47e6635505d5a13a")
            .expect("a 20-byte address");
        let action = approve_agent(Chain::Testnet, agent, "", 1_687_816_341_423);
        assert_eq!(
            signed(&vector_wallet(), &action),
            (
                padded("0x92835c1d0e54bafee2cf6134d1d04a6023778a0a5eabac00c464f040912041b0"),
                padded("0x3bc7e66258ccd3aaf0e37f91fe0ca669a5b310bbe8d45612d4a21f83b453470f"),
                28,
            )
        );
    }

    /// r, s and v only mean anything together: the recovery id is what turns
    /// them into an address, and getting it wrong hands the exchange a
    /// stranger's signature that verifies perfectly.
    #[test]
    fn a_signature_recovers_the_wallet_that_made_it() {
        let wallet = Wallet::generate();
        let action = cancel(Chain::Mainnet, &[Cancel { asset: 0, oid: 1 }], 1);
        let signature = wallet.sign(&action);
        assert_eq!(
            signature
                .recover(action.digest())
                .expect("a recoverable signature"),
            wallet.address()
        );

        // A digest the signature was not made over must recover somebody else,
        // or the check above is vacuous.
        let other = cancel(Chain::Mainnet, &[Cancel { asset: 0, oid: 2 }], 1);
        assert_ne!(
            signature
                .recover(other.digest())
                .expect("still recoverable"),
            wallet.address()
        );
    }

    /// Two keys with no shared history, because a generator that returned a
    /// constant would pass every other test in this file.
    #[test]
    fn generated_keys_differ_and_never_print_their_secret() {
        let wallet = Wallet::generate();
        assert_ne!(wallet.address(), Wallet::generate().address());

        let shown = format!("{wallet:?}");
        assert_eq!(shown, format!("Wallet({})", wallet.address()));
        assert!(
            !shown.contains("SigningKey") && !shown.contains("key"),
            "the secret must not reach a log line: {shown}"
        );
    }

    #[test]
    fn a_price_becomes_the_spelling_the_hash_expects() {
        assert_eq!(wire_num(1670.1).unwrap(), "1670.1", "no trailing zeros");
        assert_eq!(wire_num(100.0).unwrap(), "100", "and no trailing point");
        assert_eq!(wire_num(0.0147).unwrap(), "0.0147");
        assert_eq!(
            wire_num(-0.0).unwrap(),
            "0",
            "the wire has no negative zero"
        );
        assert!(
            wire_num(0.000012312312).is_err(),
            "a size the wire cannot spell must not be quietly rounded into one"
        );
        assert!(wire_num(f64::NAN).is_err());
    }

    #[test]
    fn an_address_is_only_accepted_whole() {
        assert!(Address::parse("0x5e9ee1089755c3435139848e47e6635505d5a13a").is_ok());
        assert!(
            Address::parse("5e9ee1089755c3435139848e47e6635505d5a13a").is_err(),
            "a bare hex string is not an address"
        );
        assert!(
            Address::parse("0x5e9ee1089755c3435139848e47e6635505d5a1").is_err(),
            "19 bytes would zero-pad into a different valid address"
        );
        assert!(Address::parse("0xzz").is_err());
    }

    // -----------------------------------------------------------------------
    // Live. Each of these opens its own connection and signs with a key made
    // moments earlier, so nothing it sends can execute: the signer has never
    // held a balance, an approval, or a resting order. The proof is the
    // exchange's own complaint — it recovers a signer and names it, which it
    // can only do if every byte we hashed matched what it hashed.
    // -----------------------------------------------------------------------

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("a clock after 1970")
            .as_millis() as u64
    }

    /// Posts and returns whatever came back, status and all: a 422 is the
    /// failure these tests exist to rule out, so it has to be readable rather
    /// than swallowed as a transport error.
    fn post(chain: Chain, body: &Value) -> String {
        let response = ureq::post(chain.exchange_url())
            .config()
            .http_status_as_error(false)
            .build()
            .send_json(body)
            .expect("the exchange is reachable");
        let status = response.status();
        let text = response
            .into_body()
            .read_to_string()
            .expect("a readable body");
        format!("HTTP {status} {text}")
    }

    /// Every rejection this file cares about is on the far side of
    /// deserialization: it names an address, which the exchange can only have
    /// got by recovering ours.
    fn assert_blames_the_signer(answer: &str, wallet: &Wallet, complaint: &str) {
        assert!(
            answer.contains("HTTP 200"),
            "a 4xx means the shape was wrong, not the signer: {answer}"
        );
        assert!(
            answer.contains(&wallet.address().to_string()),
            "the exchange recovered somebody else, so our hashing differs from \
             theirs: {answer}"
        );
        assert!(
            answer.contains(complaint),
            "expected an authorisation complaint, got: {answer}"
        );
    }

    #[test]
    #[ignore = "hits the live Hyperliquid API"]
    fn live_approve_agent_is_refused_for_the_signer_not_the_shape() {
        // Stands in for the master wallet, which this app never holds. It has
        // never deposited, so the approval cannot be granted.
        let master = Wallet::generate();
        let action = approve_agent(
            Chain::Mainnet,
            Wallet::generate().address(),
            "ducktape",
            now_ms(),
        );
        let answer = post(action.chain(), &action.request(&master));
        assert_blames_the_signer(&answer, &master, "Must deposit before performing actions");
    }

    #[test]
    #[ignore = "hits the live Hyperliquid API"]
    fn live_order_is_refused_for_the_signer_not_the_shape() {
        let agent = Wallet::generate();
        // Belt and braces on top of a signer that cannot trade: a buy for BTC
        // at one dollar is orders of magnitude below any book, so even in a
        // universe where this was authorised it would rest, never fill.
        let action = order(
            Chain::Mainnet,
            &[Order {
                asset: 0,
                buy: true,
                price: 1.0,
                size: 0.001,
                reduce_only: false,
                tif: Tif::Gtc,
            }],
            now_ms(),
        )
        .expect("a dollar and a millibitcoin both fit the wire");
        let answer = post(action.chain(), &action.request(&agent));
        assert_blames_the_signer(&answer, &agent, "does not exist");
    }

    #[test]
    #[ignore = "hits the live Hyperliquid API"]
    fn live_cancel_is_refused_for_the_signer_not_the_shape() {
        let agent = Wallet::generate();
        let action = cancel(Chain::Mainnet, &[Cancel { asset: 0, oid: 1 }], now_ms());
        let answer = post(action.chain(), &action.request(&agent));
        assert_blames_the_signer(&answer, &agent, "does not exist");
    }

    /// `agentName` is not in the required set, but it *is* in the signed type
    /// list — so the exchange must be hashing something in its place when the
    /// field is absent. Sending a signature made over `""` and getting our own
    /// address back is what says that something is the empty string, and it is
    /// why the builder above always sends the field rather than trusting it.
    #[test]
    #[ignore = "hits the live Hyperliquid API"]
    fn live_an_absent_agent_name_is_hashed_as_the_empty_string() {
        let master = Wallet::generate();
        let action = approve_agent(Chain::Mainnet, Wallet::generate().address(), "", now_ms());
        let mut request = action.request(&master);
        request["action"]
            .as_object_mut()
            .expect("the action is an object")
            .remove("agentName");

        let answer = post(action.chain(), &request);
        assert_blames_the_signer(&answer, &master, "Must deposit before performing actions");
    }

    /// The shape half of the same claim: drop a required field and the
    /// exchange stops *before* recovery, with a 4xx and no address in it. That
    /// is the failure the tests above are asserting they are not.
    #[test]
    #[ignore = "hits the live Hyperliquid API"]
    fn live_a_missing_field_is_refused_before_any_signer_is_recovered() {
        let wallet = Wallet::generate();
        let action = approve_agent(
            Chain::Mainnet,
            Wallet::generate().address(),
            "ducktape",
            now_ms(),
        );
        let mut request = action.request(&wallet);
        request["action"]
            .as_object_mut()
            .expect("the action is an object")
            .remove("agentAddress");

        let answer = post(action.chain(), &request);
        assert!(
            !answer.contains("HTTP 200"),
            "a missing field must not reach signature recovery: {answer}"
        );
        assert!(
            !answer.contains(&wallet.address().to_string()),
            "nothing was recovered, so nothing should be named: {answer}"
        );
    }
}
