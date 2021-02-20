use std::io;
mod config;


pub use config::{Config, ProxyTarget, ProxyTargetError};

#[non_exhaustive]
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("hyper HTTP server error: {0}")]
    Hyper(#[from] hyper::Error),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),
}
