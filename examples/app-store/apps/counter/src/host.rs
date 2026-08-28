//! What the counter asks the host for. Each is an ordinary Ice `task`: an
//! async function whose future waits on a host answer.

use app_store_sdk::host;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HostError {
    pub message: String,
}

impl From<String> for HostError {
    fn from(message: String) -> Self {
        Self { message }
    }
}

pub async fn ask_host(question: String) -> Result<String, HostError> {
    let answer = host::request("host.echo", question.as_bytes()).await?;
    String::from_utf8(answer).map_err(|error| HostError {
        message: error.to_string(),
    })
}

pub async fn wait(ms: i64) -> Result<bool, HostError> {
    host::request("clock.sleep", &ms.to_le_bytes()).await?;
    Ok(true)
}

/// Tells every app listening on the bus what the count is now, and leaves a
/// line in the store's log on the way out: a module has no stdout, so
/// `host::log` is the only `println!` it has. Nobody waits for the answer.
pub async fn publish_count(count: i64) -> Result<bool, HostError> {
    host::log(format!("count is now {count}"));
    host::request("bus.publish", format!("counter\n{count}").as_bytes()).await?;
    Ok(true)
}

pub fn question(count: i64) -> String {
    format!("The count is {count}. Still there?")
}

/// Whether the last count reached the bus — and so the Activity window, if
/// its owner has it installed.
pub fn shared_label(published: bool) -> String {
    if published {
        "Shared on the bus".into()
    } else {
        "Not shared yet".into()
    }
}

pub fn auto_label(auto: bool) -> String {
    if auto {
        "Auto: on".into()
    } else {
        "Auto: off".into()
    }
}
