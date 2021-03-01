#![deny(missing_debug_implementations)]

use std::{fmt, future::Future, io, net::SocketAddr, pin::Pin, task};

use tokio::sync::broadcast::{self, Sender};

mod config;
mod inject;
mod serve;
mod ws;

/// Reexport of `hyper` dependency (which includes `http`).
pub extern crate hyper;

pub use config::{
    Builder, Config, ConfigError, DEFAULT_CONTROL_PATH, Mount, ProxyTarget, ProxyTargetParseError
};

/// Penguin server: the main type of this library.
///
/// This type implements `Future`, and can thus be `await`ed. If you do not
/// `await` (or otherwise poll) this, the server will not start serving.
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Server {
    // TODO: maybe avoid boxing this if possible?
    future: Pin<Box<dyn Send + Future<Output = Result<(), Error>>>>,
}

impl Server {
    /// Returns a builder to configure the server with the bind address of the
    /// server being set to `addr`.
    pub fn bind(addr: SocketAddr) -> Builder {
        Builder::new(addr)
    }

    /// Builds a server and a controller from a configuration. Most of the time
    /// you can use [`Builder::build`] instead of this method.
    pub fn build(config: Config) -> (Self, Controller) {
        let (sender, _) = broadcast::channel(ACTION_CHANNEL_SIZE);
        let controller = Controller(sender.clone());
        let future = Box::pin(serve::run(config, sender));

        (Self { future }, controller)
    }
}

impl Future for Server {
    type Output = Result<(), Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        self.future.as_mut().poll(cx)
    }
}

impl fmt::Debug for Server {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad("Server(_)")
    }
}

const ACTION_CHANNEL_SIZE: usize = 8;

/// A handle to send commands to the server.
#[derive(Debug, Clone)]
pub struct Controller(Sender<Action>);

impl Controller {
    /// Reloads all active browser sessions.
    pub fn reload(&self) {
        let _ = self.0.send(Action::Reload);
    }

    /// Shows a message as overlay in all active browser sessions. The given
    /// string will be copied into the `innerHTML` of a `<div>` verbatim.
    ///
    /// This call will overwrite/hide all previous messages.
    pub fn show_message(&self, msg: impl Into<String>) {
        let _ = self.0.send(Action::Message(msg.into()));
    }
}

/// Error returned by awaiting `Server`: everything that can go wrong when
/// running the server.
#[non_exhaustive]
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("hyper HTTP server error: {0}")]
    Hyper(#[from] hyper::Error),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Websocket error: {0}")]
    Tungestine(#[from] hyper_tungstenite::tungstenite::Error),
}

#[derive(Debug, Clone)]
enum Action {
    Reload,
    Message(String),
}
