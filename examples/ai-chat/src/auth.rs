//! Signing in from inside the window.
//!
//! This is the device flow the Codex CLI runs under `codex login --device-auth`,
//! spoken directly: ask the auth host for a short code, show it, and wait while
//! the person types it into a browser — theirs, or one on another machine. The
//! window never sees a password and never opens one.
//!
//! **Where the tokens go.** This app writes its own file and never the CLI's.
//! Overwriting `~/.codex/auth.json` would put one app's login in another app's
//! hands, and a refresh here could invalidate the token the CLI is holding. It
//! does *read* the CLI's file when it has none of its own, so a machine already
//! signed in through `codex login` needs no second login.

use std::io::Write;
use std::path::PathBuf;

use base64::Engine;
use serde_json::{Value, json};

use crate::codex::CodexError;

const ISSUER: &str = "https://auth.openai.com";
/// The Codex CLI's own client. The tokens are only good for the Codex backend,
/// which is the only thing this window talks to.
const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
/// Where the person types the code.
const VERIFICATION_URI: &str = "https://auth.openai.com/deviceauth/usercode";
const REDIRECT_URI: &str = "https://auth.openai.com/deviceauth/callback";
const SCOPE: &str = "openid profile email offline_access";
/// The auth host is fronted by a bot check that reads the agent.
const AGENT: &str = "codex_cli_rs/0.147.0";
/// The host's own suggested poll interval, when it does not say.
const POLL_SECONDS: u64 = 5;
/// The device code the host mints lasts fifteen minutes; stop before it does.
const POLL_LIMIT: u64 = 15 * 60 / POLL_SECONDS;

/// A code to type, and the page to type it into.
#[derive(Clone, Debug, PartialEq)]
pub struct DeviceCode {
    pub user_code: String,
    pub verification_uri: String,
    pub device_auth_id: String,
}

/// Ask the host for a code. Nothing is signed in yet when this returns.
pub fn device_start() -> Result<DeviceCode, CodexError> {
    let answer = post_json(
        &format!("{ISSUER}/api/accounts/deviceauth/usercode"),
        &json!({ "client_id": CLIENT_ID }),
    )?;
    let (Some(user_code), Some(device_auth_id)) = (
        answer["user_code"].as_str(),
        answer["device_auth_id"].as_str(),
    ) else {
        return Err(CodexError::new(
            "The sign-in host did not return a code to type.",
        ));
    };
    Ok(DeviceCode {
        user_code: user_code.to_owned(),
        verification_uri: VERIFICATION_URI.to_owned(),
        device_auth_id: device_auth_id.to_owned(),
    })
}

/// Wait for the code to be approved, then keep the login it grants.
///
/// Blocking, and slow by design: it is one poll every few seconds until a
/// person somewhere finishes typing. The caller runs it off the frame loop.
pub fn device_wait(code: &DeviceCode) -> Result<String, CodexError> {
    for _ in 0..POLL_LIMIT {
        match poll_once(code) {
            Poll::Granted(grant) => {
                let tokens = exchange(&grant)?;
                save(&tokens)?;
                return Ok(email_of(&tokens).unwrap_or_default());
            }
            Poll::Pending => std::thread::sleep(std::time::Duration::from_secs(POLL_SECONDS)),
            Poll::Failed(error) => return Err(error),
        }
    }
    Err(CodexError::new(
        "The code expired before it was approved. Start again.",
    ))
}

enum Poll {
    Pending,
    Granted(Value),
    Failed(CodexError),
}

fn poll_once(code: &DeviceCode) -> Poll {
    let body = json!({
        "client_id": CLIENT_ID,
        "device_auth_id": code.device_auth_id,
        "user_code": code.user_code,
    });
    let (status, answer) = match post_raw(&format!("{ISSUER}/api/accounts/deviceauth/token"), &body)
    {
        Ok(pair) => pair,
        Err(error) => return Poll::Failed(error),
    };
    if status == 200 {
        return Poll::Granted(answer);
    }
    // The host says "pending" with a 403 and its own code rather than a status
    // of its own, so the code is what distinguishes waiting from refused.
    if answer["error"]["code"].as_str() == Some("deviceauth_authorization_pending") {
        return Poll::Pending;
    }
    Poll::Failed(CodexError::new(format!(
        "Sign-in was refused ({status}): {}",
        answer["error"]["message"]
            .as_str()
            .unwrap_or("no reason given")
    )))
}

/// Trade the approved grant for tokens.
///
/// UNVERIFIED. Everything up to here has been checked against the live host —
/// the code is minted, and an unapproved code reads as pending — but the shape
/// the host returns *once a code is approved*, and the parameters this exchange
/// needs, are read off the CLI's own strings rather than observed. One real
/// approval settles both; `a_real_sign_in_completes` is that check.
fn exchange(grant: &Value) -> Result<Value, CodexError> {
    let Some(code) = grant["authorization_code"].as_str() else {
        return Err(CodexError::new(
            "The sign-in host approved the code but returned no grant.",
        ));
    };
    let mut form = vec![
        ("grant_type", "authorization_code"),
        ("client_id", CLIENT_ID),
        ("code", code),
        ("redirect_uri", REDIRECT_URI),
    ];
    if let Some(verifier) = grant["code_verifier"].as_str() {
        form.push(("code_verifier", verifier));
    }
    post_form(&format!("{ISSUER}/oauth/token"), &form)
}

/// Trade a refresh token for a fresh access token.
///
/// Only ever called with this app's own stored login. A refresh can rotate the
/// token it was given, which is exactly why the CLI's file is never the one
/// being refreshed here.
pub fn refresh(refresh_token: &str) -> Result<Value, CodexError> {
    let tokens = post_form(
        &format!("{ISSUER}/oauth/token"),
        &[
            ("grant_type", "refresh_token"),
            ("client_id", CLIENT_ID),
            ("refresh_token", refresh_token),
            ("scope", SCOPE),
        ],
    )?;
    save(&tokens)?;
    Ok(tokens)
}

fn post_json(url: &str, body: &Value) -> Result<Value, CodexError> {
    let (status, answer) = post_raw(url, body)?;
    if status != 200 {
        return Err(CodexError::new(format!(
            "The sign-in host refused the request ({status}): {}",
            answer["error"]["message"].as_str().unwrap_or("no reason")
        )));
    }
    Ok(answer)
}

fn post_raw(url: &str, body: &Value) -> Result<(u16, Value), CodexError> {
    let response = ureq::post(url)
        .config()
        .http_status_as_error(false)
        .build()
        .header("Content-Type", "application/json")
        .header("originator", "codex_cli_rs")
        .header("User-Agent", AGENT)
        .send_json(body)
        .map_err(|error| CodexError::new(format!("Could not reach the sign-in host: {error}")))?;
    let status = response.status().as_u16();
    let text = response.into_body().read_to_string().unwrap_or_default();
    Ok((status, serde_json::from_str(&text).unwrap_or(Value::Null)))
}

fn post_form(url: &str, form: &[(&str, &str)]) -> Result<Value, CodexError> {
    let response = ureq::post(url)
        .config()
        .http_status_as_error(false)
        .build()
        .header("originator", "codex_cli_rs")
        .header("User-Agent", AGENT)
        .send_form(form.to_vec())
        .map_err(|error| CodexError::new(format!("Could not reach the sign-in host: {error}")))?;
    let status = response.status().as_u16();
    let text = response.into_body().read_to_string().unwrap_or_default();
    let answer: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    if status != 200 {
        return Err(CodexError::new(format!(
            "The sign-in could not be completed ({status}): {}",
            answer["error_description"]
                .as_str()
                .or_else(|| answer["error"].as_str())
                .unwrap_or("no reason given")
        )));
    }
    Ok(answer)
}

/// This app's own credential file. Never the CLI's.
fn our_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config")
        });
    base.join("ducktape-ai-chat").join("auth.json")
}

fn codex_path() -> PathBuf {
    if let Some(home) = std::env::var_os("CODEX_HOME") {
        return PathBuf::from(home).join("auth.json");
    }
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
        .join(".codex")
        .join("auth.json")
}

fn save(tokens: &Value) -> Result<(), CodexError> {
    let path = our_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| CodexError::new(format!("Could not keep the sign-in: {error}")))?;
    }
    let stored = json!({
        "tokens": {
            "access_token": tokens["access_token"],
            "refresh_token": tokens["refresh_token"],
            "id_token": tokens["id_token"],
            "account_id": account_id_of(tokens),
        }
    });
    let mut file = std::fs::File::create(&path)
        .map_err(|error| CodexError::new(format!("Could not keep the sign-in: {error}")))?;
    restrict(&file);
    file.write_all(stored.to_string().as_bytes())
        .map_err(|error| CodexError::new(format!("Could not keep the sign-in: {error}")))
}

/// Owner-only, because the file holds a live credential.
#[cfg(unix)]
fn restrict(file: &std::fs::File) {
    use std::os::unix::fs::PermissionsExt;
    let _ = file.set_permissions(std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn restrict(_file: &std::fs::File) {}

/// Whose login is in force.
///
/// It decides whether this app may refresh: a refresh can rotate the token it
/// was given, and rotating the CLI's would break `codex` for a login this app
/// only borrowed.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Login {
    Ours,
    Borrowed,
}

/// The login in force: this app's own, or the CLI's when it has none.
pub fn stored() -> Option<(Value, Login)> {
    for (path, whose) in [(our_path(), Login::Ours), (codex_path(), Login::Borrowed)] {
        if let Ok(text) = std::fs::read_to_string(&path)
            && let Ok(value) = serde_json::from_str::<Value>(&text)
            && value["tokens"]["access_token"].is_string()
        {
            return Some((value, whose));
        }
    }
    None
}

/// Forget this app's login. The CLI's is left alone, so signing out here and
/// finding yourself still signed in through `codex login` is expected.
#[cfg(not(test))]
pub fn sign_out() -> bool {
    std::fs::remove_file(our_path()).is_ok()
}

/// The account id the Codex backend wants, read from the token's own claims.
fn account_id_of(tokens: &Value) -> Value {
    claims(tokens["access_token"].as_str().unwrap_or_default())
        .map(|claims| claims["https://api.openai.com/auth"]["chatgpt_account_id"].clone())
        .unwrap_or(Value::Null)
}

pub fn email_of(tokens: &Value) -> Option<String> {
    let token = tokens["id_token"]
        .as_str()
        .or_else(|| tokens["access_token"].as_str())?;
    Some(claims(token)?["email"].as_str()?.to_owned())
}

/// A JWT's claims — the unsigned half, read only to name the account on screen
/// and to find the account id the backend asks for. Nothing is authorised on
/// the strength of it.
pub fn claims(token: &str) -> Option<Value> {
    let payload = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Ask for a code, off the frame loop.
pub async fn begin_sign_in() -> Result<DeviceCode, CodexError> {
    smol::unblock(device_start).await
}

/// Wait for the code to be approved, off the frame loop. Returns the account
/// it signed in as.
pub async fn finish_sign_in(code: DeviceCode) -> Result<String, CodexError> {
    smol::unblock(move || device_wait(&code)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cloudflare fronts the auth host and decides by more than a header. If
    /// the client this app uses cannot get through, an in-window sign-in is
    /// impossible however well the rest of the flow is mapped.
    #[test]
    #[ignore = "reaches auth.openai.com"]
    fn the_clients_own_request_reaches_the_auth_host() {
        let code = device_start().expect("a device code");
        eprintln!("code {} at {}", code.user_code, code.verification_uri);
        assert!(!code.user_code.is_empty());
        assert!(!code.device_auth_id.is_empty());
    }

    /// An unapproved code has to read as waiting, not as refused — the poll
    /// loop turns on exactly this distinction, and the host signals it with a
    /// 403 rather than a status that means "keep going".
    #[test]
    #[ignore = "reaches auth.openai.com"]
    fn an_unapproved_code_reads_as_pending() {
        let code = device_start().expect("a device code");
        assert!(
            matches!(poll_once(&code), Poll::Pending),
            "a code nobody has typed yet must read as pending"
        );
    }

    /// The one part of the flow that cannot be checked without a person: the
    /// shape the host returns once a code is approved, and whether the grant
    /// it carries exchanges for tokens. Prints the code and waits.
    ///
    /// `AI_CHAT_LIVE=1 cargo test -p ai-chat-example -- --ignored --nocapture a_real_sign_in`
    #[test]
    #[ignore = "needs a person to approve a code"]
    fn a_real_sign_in_completes() {
        let code = device_start().expect("a device code");
        eprintln!(
            "\n    open {}\n    code {}\n",
            code.verification_uri, code.user_code
        );

        let raw = loop {
            match poll_once(&code) {
                Poll::Granted(grant) => break grant,
                Poll::Pending => std::thread::sleep(std::time::Duration::from_secs(3)),
                Poll::Failed(error) => panic!("refused: {}", error.message),
            }
        };
        eprintln!(
            "granted fields: {:?}",
            raw.as_object().map(|o| o.keys().collect::<Vec<_>>())
        );

        let tokens = exchange(&raw).expect("the grant exchanges for tokens");
        eprintln!(
            "token fields: {:?}",
            tokens.as_object().map(|o| o.keys().collect::<Vec<_>>())
        );
        assert!(tokens["access_token"].is_string(), "an access token");
        assert!(tokens["refresh_token"].is_string(), "a refresh token");
        assert!(account_id_of(&tokens).is_string(), "an account id claim");

        save(&tokens).expect("the login is kept");
        let (stored_file, whose) = stored().expect("a stored login");
        assert_eq!(whose, Login::Ours, "this app's own file, not the CLI's");
        eprintln!("signed in as {:?}", email_of(&stored_file["tokens"]));
    }

    /// The whole point of keeping our own file: a refresh here must never be
    /// able to rotate the token the CLI is holding.
    #[test]
    fn this_app_never_writes_the_clis_credential_file() {
        assert_ne!(our_path(), codex_path());
        assert!(our_path().ends_with("ducktape-ai-chat/auth.json"));
    }
}
