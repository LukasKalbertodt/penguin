use futures::{SinkExt, StreamExt};
use hyper_tungstenite::{HyperWebsocket, tungstenite::{Error, Message, error::ProtocolError}};
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
                            "Missed {} actions. Did you submit too many actions too quickly? \
                                For example, this can happen by watching a directory where lots \
                                of files change at the same time.",
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

                    // The library tungstenite already handles ping requests
                    // internally, but we still have to "call into the library"
                    // for the pong packet to actually get sent.
                    Some(Ok(Message::Ping(_))) => {
                        match websocket.flush().await {
                            // We explicitly ignore a couple of errors related
                            // to a closed connection. If the connection is
                            // closed, we do not care that our pong send failed.
                            Ok(_)
                            | Err(Error::ConnectionClosed)
                            | Err(Error::AlreadyClosed)
                            | Err(Error::Protocol(ProtocolError::ResetWithoutClosingHandshake))
                            | Err(Error::Protocol(ProtocolError::SendAfterClosing)) => {}

                            Err(e) => log::warn!("Error sending pong WS packet: {}", e),
                        }
                    }

                    // We catch this particular error since it happens a lot
                    // when a tab is reloading and the WS connection isn't
                    // properly closed. This is nothing to worry about and we
                    // just stop/drop the connection.
                    Some(Err(Error::Protocol(ProtocolError::ResetWithoutClosingHandshake))) => {
                        break;
                    }

                    // All other errors get shown as warnings.
                    Some(Err(e)) => {
                        log::warn!(
                            "Error receiving WS message. Shutting down WS connection. Error: {}",
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
