//! What the counter asks the host for. Each is an ordinary Ice `task`: an
//! async function whose future waits on a host response.

use app_store_sdk::host;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HostError {
    pub message: String,
}

pub async fn ask_host(question: String) -> Result<String, HostError> {
    let answer = host::request("echo", question.as_bytes()).await;
    String::from_utf8(answer).map_err(|error| HostError {
        message: error.to_string(),
    })
}

pub async fn wait(ms: i64) -> Result<bool, HostError> {
    host::request("sleep", &ms.to_le_bytes()).await;
    Ok(true)
}

pub fn question(count: i64) -> String {
    format!("The count is {count}. Still there?")
}

pub fn auto_label(auto: bool) -> String {
    if auto {
        "Auto: on".into()
    } else {
        "Auto: off".into()
    }
}
