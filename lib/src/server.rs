use std::{convert::Infallible, future::Future, panic::AssertUnwindSafe, sync::Arc};

use futures::{FutureExt, SinkExt, StreamExt};
use hyper::{Body, Method, Request, Response, Server, StatusCode, service::{make_service_fn, service_fn}};
use hyper_tungstenite::{HyperWebsocket, tungstenite::Message};
use tokio::sync::broadcast::{Receiver, Sender, error::RecvError};

use super::{Action, Config, Error, fileserver, proxy};


pub(crate) async fn run(config: Config, actions: Sender<Action>) -> Result<(), Error> {
    let addr = config.bind_addr;

    let config = Arc::new(config);
    let make_service = make_service_fn(move |_| {
        let config = Arc::clone(&config);
        let actions = actions.clone();

        async {
            Ok::<_, Infallible>(service_fn(move |req| {
                handle_internal_errors(
                    handle(req, Arc::clone(&config), actions.clone())
                )
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_service);

    server.await?;

    Ok(())
}

async fn handle_internal_errors(
    future: impl Future<Output = Response<Body>>,
) -> Result<Response<Body>, Infallible> {
    fn internal_server_error(msg: &str) -> Response<Body> {
        let body = format!("Internal server error: this is a bug in Penguin!\n\n{}\n", msg);
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(body.into())
            .unwrap()
    }

    // The `AssertUnwindSafe` is unfortunately necessary. The whole story of
    // unwind safety is strange. What we are basically saying here is: "if the
    // future panicks, the global/remaining application state is not 'broken'.
    // It is safe to continue with the program in case of a panic."
    match AssertUnwindSafe(future).catch_unwind().await {
        Ok(response) => Ok(response),
        Err(panic) => {
            // The `panic` information is just an `Any` object representing the
            // value the panic was invoked with. For most panics (which use
            // `panic!` like `println!`), this is either `&str` or `String`.
            let msg = panic.downcast_ref::<String>()
                .map(|s| s.as_str())
                .or(panic.downcast_ref::<&str>().map(|s| *s));

            Ok(internal_server_error(msg.unwrap_or("panic")))
        }
    }
}

/// Handles a single incoming request.
async fn handle(
    req: Request<Body>,
    config: Arc<Config>,
    actions: Sender<Action>,
) -> Response<Body> {
    if req.uri().path().starts_with(&config.control_path) {
        handle_control(req, config, actions).await
    } else if let Some(response) = fileserver::try_serve(&req, &config).await {
        response
    } else if let Some(proxy) = &config.proxy {
        proxy::forward(req, proxy, config.clone()).await
    } else {
        not_found()
    }
}

/// Handles "control requests", i.e. request to the control path.
async fn handle_control(
    req: Request<Body>,
    config: Arc<Config>,
    actions: Sender<Action>,
) -> Response<Body> {
    if hyper_tungstenite::is_upgrade_request(&req) {
        match hyper_tungstenite::upgrade(req, None) {
            Ok((response, websocket)) => {
                // Spawn a task to handle the websocket connection.
                tokio::spawn(async move {
                    let receiver = actions.subscribe();
                    if let Err(e) = handle_websocket(websocket, receiver).await {
                        // TODO
                        eprintln!("Error in websocket connection: {}", e);
                    }
                });

                // Return the response so the spawned future can continue.
                response
            }
            Err(_) => {
                // TODO: `upgrade` does not guarantee this (yet), but from
                // looking at the code, I think an error here means that the
                // request is invalid.

                bad_request("Failed to upgrade to WS connection\n")
            }
        }
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

            (&Method::POST, "/message") => {
                let (_, body) = req.into_parts();
                let body = hyper::body::to_bytes(body)
                    .await
                    .expect("failed to download message body");

                match std::str::from_utf8(&body) {
                    Err(_) => bad_request("Bad request: request body is not UTF8\n"),
                    Ok(s) => {
                        // We ignore errors here: if there are no receivers, so be it.
                        // Although we might want to include the number of receivers in
                        // the event.
                        let _ = actions.send(Action::Message(s.into()));
                        // TODO: event

                        Response::new(Body::empty())
                    }
                }
            }

            _ => bad_request("Invalid request to libpenguin control path\n"),
        }
    }
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
                    Ok(Action::Reload) => "reload".to_string(),
                    Ok(Action::Message(msg)) => format!("message\n{}", msg),
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

pub(crate) fn bad_request(msg: impl Into<Body>) -> Response<Body> {
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(msg.into())
        .expect("bug: invalid response")
}

pub(crate) fn not_found() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("Not found\n"))
        .expect("bug: invalid response")
}
