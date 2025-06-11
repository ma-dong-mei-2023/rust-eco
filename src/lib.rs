pub mod runtime;
pub mod time;
pub mod file;
pub mod socket;
pub mod http;
pub mod sync;
pub mod channel;
pub mod dns;
pub mod sys;
pub mod log;
pub mod protocols;

pub use runtime::{Eco, spawn, sleep, current, yield_now};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug, thiserror::Error)]
pub enum EcoError {
    #[error("Runtime error: {0}")]
    Runtime(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Timeout")]
    Timeout,
    #[error("Cancelled")]
    Cancelled,
}