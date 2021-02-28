use futures::{SinkExt, StreamExt};
use hyper_tungstenite::{HyperWebsocket, tungstenite::Message};
use tokio::sync::broadcast::{Receiver, error::RecvError};

use super::Action;


/// Function to handle a single websocket (listen for incoming `Action`s and
/// stop if the WS connection is closed). There is one task per WS connection.
pub(crate) async fn handle_connection(
    websocket: HyperWebsocket,
    mut actions: Receiver<Action>,
) {
    let mut websocket = match websocket.await {
        Ok(ws) => ws,
        Err(e) => {
            log::warn!("failed to establish websocket connection: {}", e);
            return;
        }
    };

    loop {
        tokio::select! {
            action = actions.recv() => {
                let data = match &action {
                    // When all senders have closed, there is no reason to
                    // continue keeping this task alive.
                    Err(RecvError::Closed) => break,
                    Err(RecvError::Lagged(skipped)) => {
                        // I really can't imagine this happening: this would
                        // mean this WS task was never awoken while many actions
                        // were incoming.
                        log::warn!(
                            "Missed {} actions. This should not happen. \
                                If you see this, please open an issue here: \
                                https://github.com/LukasKalbertodt/penguin/issues",
                            skipped,
                        );
                        continue;
                    }
                    Ok(Action::Reload) => {
                        log::trace!("Sending reload WS command");
                        "reload".to_string()
                    }
                    Ok(Action::Message(msg)) => {
                        log::trace!("Sending message WS command");
                        format!("message\n{}", msg)
                    }
                };

                if let Err(e) = websocket.send(Message::text(data)).await {
                    log::warn!("Failed to send WS message for action '{:?}': {}", action, e);
                }
            }

            message = websocket.next() => {
                match message {
                    // If the WS connection was closed, we can just stop this
                    // function.
                    None | Some(Ok(Message::Close(_))) => break,

                    Some(Err(e)) => {
                        log::warn!(
                            "Error receiving unexpected WS message. Shutting down \
                                WS connection. Error: {}",
                            e,
                        );
                        break;
                    }

                    _ => log::warn!("unexpected incoming WS message {:?}", message),
                }
            }
        };
    }
}
