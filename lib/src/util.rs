//! Utility functions.

use std::time::Duration;

use hyper::http::uri::Scheme;
use tokio::net::TcpStream;

use crate::ProxyTarget;


/// Repeatedly tries to connect to the given proxy via TCP, returning once a
/// connection was established.
///
/// After each attempt, this function waits for `poll_period` before trying
/// again. If the proxy target is never reachable, this function's future never
/// resolves. To stop this function after some timeout, use some external
/// functions, e.g. `tokio::time::timeout`.
///
/// This function can be used just before calling
/// [`Controller::reload`][super::Controller::reload] to make sure the proxy
/// server is ready. This avoids the user seeing "Cannot reach proxy target".
pub async fn wait_for_proxy(target: &ProxyTarget, poll_period: Duration) {
    let mut interval = tokio::time::interval(poll_period);
    let port = target.authority
        .port_u16()
        .unwrap_or(if target.scheme == Scheme::HTTP { 80 } else { 443 });

    loop {
        if TcpStream::connect((target.authority.host(), port)).await.is_ok() {
            break;
        }
        interval.tick().await;
    }
}
