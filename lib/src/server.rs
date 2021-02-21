use std::{convert::Infallible, sync::Arc};

use futures::{SinkExt, StreamExt};
use hyper::{Body, Method, Request, Response, Server, StatusCode, service::{make_service_fn, service_fn}};
use hyper_tungstenite::{HyperWebsocket, tungstenite::Message};
use tokio::sync::broadcast::{Receiver, Sender, error::RecvError};

use super::{Action, Config, Error, proxy};


pub(crate) async fn run(config: Config, actions: Sender<Action>) -> Result<(), Error> {
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
    config: Arc<Config>,
    actions: Sender<Action>,
) -> Result<Response<Body>, Error> {
    let response = if hyper_tungstenite::is_upgrade_request(&req) {
        let (response, websocket) = hyper_tungstenite::upgrade(req, None)?;

        // Spawn a task to handle the websocket connection.
        tokio::spawn(async move {
            let receiver = actions.subscribe();
            if let Err(e) = handle_websocket(websocket, receiver).await {
                eprintln!("Error in websocket connection: {}", e);
            }
        });

        // Return the response so the spawned future can continue.
        response
    } else {
        let subpath = req.uri().path().strip_prefix(&config.control_path).unwrap();
        match (req.method(), subpath) {
            (&Method::POST, "/reload") => {
                // We ignore errors here: if there are no receivers, so be it.
                // Although we might want to include the number of receivers in
                // the event.
                let _ = actions.send(Action::Reload);
                // TODO: event

                Response::new(Body::empty())
            }

            _ => {
                Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from("Invalid request to libpenguin control path"))
                    .expect("bug: invalid response")
            }
        }
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
                match message {
                    Some(Err(e)) => Err(e)?,

                    // If the WS connection was closed, we can just stop this
                    // function.
                    None | Some(Ok(Message::Close(_))) => break,

                    _ => {
                        // TODO
                        println!("warn: unexpected message {:?}", message);
                    }
                }
            }
        };
    }

    Ok(())
}
