use std::{future::Future, io};

use tokio::sync::broadcast::{self, Sender};

mod config;
mod fileserver;
mod inject;
mod proxy;
mod server;


pub use config::{Config, ProxyTarget, ProxyTargetError};

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

const ACTION_CHANNEL_SIZE: usize = 4;

/// Main entry point of this library: returns a controller and a future that
/// represent the server.
///
/// You have to poll the future (usually by `await`ing it) in order for the
/// server to actually listen and serve requests. The controller can be used to
/// send various control signals.
pub fn serve(
    config: Config,
) -> Result<(Controller, impl Future<Output = Result<(), Error>>), Error> {
    let (sender, _) = broadcast::channel(ACTION_CHANNEL_SIZE);
    let controller = Controller(sender.clone());
    let future = server::run(config, sender);

    Ok((controller, future))
}
