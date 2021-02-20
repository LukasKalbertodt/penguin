use std::{convert::Infallible, future::Future, io, sync::Arc};

use futures::{SinkExt, StreamExt};
use hyper::{Body, Request, Response, Server, StatusCode, service::{make_service_fn, service_fn}};
use hyper_tungstenite::{HyperWebsocket, tungstenite::Message};
use tokio::sync::broadcast::{self, Receiver, Sender, error::RecvError};

mod config;
mod inject;
mod proxy;


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
}

#[derive(Debug, Clone)]
enum Action {
    Reload,
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
    let future = run(config, sender);

    Ok((controller, future))
}


async fn run(config: Config, actions: Sender<Action>) -> Result<(), Error> {
    let addr = config.bind_addr;

    let config = Arc::new(config);
    let make_service = make_service_fn(move |_| {
        let config = Arc::clone(&config);
        let actions = actions.clone();

        async {
            Ok::<_, Infallible>(service_fn(move |req| {
                handle(req, Arc::clone(&config), actions.clone())
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_service);

    server.await?;

    Ok(())
}

/// Handles a single incoming request.
async fn handle(
    req: Request<Body>,
    config: Arc<Config>,
    actions: Sender<Action>,
) -> Result<Response<Body>, Error> {
    let response = if req.uri().path().starts_with(&config.control_path) {
        handle_control(req, config, actions).await?
    } else if let Some(proxy) = &config.proxy {
        proxy::forward(req, proxy, config.clone()).await?
    } else {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not found"))
            .expect("bug: invalid response")
    };

    Ok(response)
}

/// Handles "control requests", i.e. request to the control path.
async fn handle_control(
    req: Request<Body>,
    _config: Arc<Config>,
    actions: Sender<Action>,
) -> Result<Response<Body>, Error> {
    let response = if hyper_tungstenite::is_upgrade_request(&req) {
        let (response, websocket) = hyper_tungstenite::upgrade(req, None)?;

        // Spawn a task to handle the websocket connection.
        tokio::spawn(async move {
            let receiver = actions. subscribe();
            if let Err(e) = handle_websocket(websocket, receiver).await {
                eprintln!("Error in websocket connection: {}", e);
            }
        });

        // Return the response so the spawned future can continue.
        response
    } else {
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from("Invalid request to libpenguin control path"))
            .expect("bug: invalid response")
    };

    Ok(response)
}

/// Function to handle a single websocket (listen for incoming `Action`s and
/// stop if the WS connection is closed). There is one task per WS connection.
async fn handle_websocket(
    websocket: HyperWebsocket,
    mut actions: Receiver<Action>,
) -> Result<(), Error> {
    let mut websocket = websocket.await?;

    loop {
        tokio::select! {
            action = actions.recv() => {
                let data = match action {
                    // When all senders have closed, there is no reason to
                    // continue keeping this task alive.
                    Err(RecvError::Closed) => break,
                    Err(RecvError::Lagged(_)) => {
                        // TODO: handle this somehow?
                        continue;
                    }
                    Ok(Action::Reload) => "reload",
                };

                websocket.send(Message::text(data)).await?;
            }

            message = websocket.next() => {
                if message.is_none() {
                    // TODO: notify websocket close
                    break;
                }

                println!("msg :("); // TODO
            }
        };
    }

    Ok(())
}
