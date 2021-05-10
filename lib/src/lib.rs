//! Penguin is a dev server with features like auto-reloading, a static file
//! server, and proxy-support. It is available both, as an app and as a library.
//! You are currently reading the library docs. If you are interested in the CLI
//! app, see [the README](https://github.com/LukasKalbertodt/penguin#readme).
//!
//! This library essentially allows you to configure and then start an HTTP
//! server. After starting the server you get a [`Controller`] which allows you
//! to send commands to active browser sessions, like reloading the page or
//! showing a message.
//!
//!
//! # Quick start
//!
//! This should get you started as it shows almost everything this library has
//! to offer:
//!
//! ```no_run
//! use std::{path::Path, time::Duration};
//! use penguin::Server;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Configure the server.
//!     let (server, controller) = Server::bind(([127, 0, 0, 1], 4090).into())
//!         .proxy("localhost:8000".parse()?)
//!         .add_mount("/assets", Path::new("./frontend/build"))?
//!         .build()?;
//!
//!     // In some other task, you can control the browser sessions. This dummy
//!     // code just waits 5 seconds and then reloads all sessions.
//!     tokio::spawn(async move {
//!         tokio::time::sleep(Duration::from_secs(5)).await;
//!         controller.reload();
//!     });
//!
//!     server.await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Routing
//!
//! Incoming requests are routed like this (from highest to lowest priority):
//!
//! - Requests to the control path (`/~~penguin` by default) are internally
//!   handled. This is used for establishing WS connections and to receive
//!   commands.
//! - Requests with a path matching one of the mounts is served from that
//!   directory.
//!     - The most specific mount (i.e. the one with the longest URI path) is
//!       used. Consider there are two mounts: `/cat` -> `./foo` and `/cat/paw`
//!       -> `./bar`. Then a request to `/cat/paw/info.json` is replied to with
//!       `./bar/info.json` while a request to `/cat/style.css` is replied to
//!       with `./foo/style.css`
//! - If a proxy is configured, then all remaining requests are forwarded to it
//!   and its reply is forwarded back to the initiator of the request. Otherwise
//!   (no proxy configured), all remaining requests are answered with 404.
//!
//!

#![deny(missing_debug_implementations)]

use std::{fmt, future::Future, net::SocketAddr, pin::Pin, task};

use tokio::sync::broadcast::{self, Sender};

mod config;
mod inject;
mod serve;
pub mod util;
mod ws;

#[cfg(test)]
mod tests;

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
    future: Pin<Box<dyn Send + Future<Output = Result<(), hyper::Error>>>>,
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
    type Output = Result<(), hyper::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        self.future.as_mut().poll(cx)
    }
}

impl fmt::Debug for Server {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad("Server(_)")
    }
}

const ACTION_CHANNEL_SIZE: usize = 64;

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

#[derive(Debug, Clone)]
enum Action {
    Reload,
    Message(String),
}
