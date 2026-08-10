#[derive(Clone, Debug, Hash, PartialEq)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub done: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppError {
    pub message: String,
}

#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod secret;
mod shader;
mod tasks;

#[cfg(test)]
pub use fixtures::*;
#[cfg(test)]
pub use secret::*;
pub use shader::*;
pub use tasks::*;
