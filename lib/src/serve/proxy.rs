use std::{cmp::min, sync::{Arc, atomic::{AtomicBool, Ordering}}, time::Duration};

use hyper::{Body, Client, Request, Response, StatusCode, Uri, header::{self, HeaderValue}};
use hyper_tls::HttpsConnector;
use tokio::sync::broadcast::Sender;

use crate::{Action, Config, ProxyTarget, inject};

use super::{Context, SERVER_HEADER};


/// HTML content to reply in case an error occurs when connecting to the proxy.
const PROXY_ERROR_HTML: &str = include_str!("../assets/proxy-error.html");

pub(crate) struct ProxyContext {
    is_polling_target: Arc<AtomicBool>,
}

impl ProxyContext {
    pub(crate) fn new() -> Self {
        Self {
            is_polling_target: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// Forwards the given request to the specified proxy target and returns its
/// response.
///
/// If the proxy target cannot be reached, a 502 Bad Gateway or 504 Gateway
/// Timeout response is returned.
pub(crate) async fn forward(
    mut req: Request<Body>,
    target: &ProxyTarget,
    ctx: &Context,
    actions: Sender<Action>,
) -> Response<Body> {
    // Build new URI and change the given request.
    let uri = {
        let mut parts = req.uri().clone().into_parts();
        parts.scheme = Some(target.scheme.clone());
        parts.authority = Some(target.authority.clone());
        Uri::from_parts(parts).expect("bug: invalid URI")
    };
    *req.uri_mut() = uri.clone();

    // If the `host` header is set, we need to adjust it.
    if let Some(host) = req.headers_mut().get_mut("host") {
        // `http::Uri` already does not parse non-ASCII hosts. Unicode hosts
        // have to be encoded as punycode.
        *host = HeaderValue::from_str(target.authority.as_str())
            .expect("bug: URI authority should be ASCII");
    }

    log::trace!("Forwarding request to proxy target {}", uri);
    let client = Client::builder().build::<_, hyper::Body>(HttpsConnector::new());
    match client.request(req).await {
        Ok(response) => {
            let content_type = response.headers().get(header::CONTENT_TYPE);
            if content_type.map_or(false, |v| v.as_ref().starts_with(b"text/html")) {
                log::trace!("Response from proxy is HTML: injecting script");

                // The response is HTML: we need to download it completely and
                // inject our script.
                let (parts, body) = response.into_parts();
                let body = match hyper::body::to_bytes(body).await {
                    Ok(body) => body,
                    Err(e) => {
                        log::warn!("Failed to download full response from proxy target");
                        let msg = format!("Failed to download response from {}\n\n{}", uri, e);
                        return gateway_error(&msg, e, &ctx.config);
                    }
                };

                let new_body = inject::into(&body, &ctx.config);
                let new_len = new_body.len();
                let new_body = Body::from(new_body);

                let mut response = Response::from_parts(parts, new_body);
                if let Some(content_len) = response.headers_mut().get_mut(header::CONTENT_LENGTH) {
                    *content_len = new_len.into();
                }
                response
            } else {
                response
            }
        }

        Err(e) => {
            log::warn!("Failed to reach proxy target");
            let msg = format!("Failed to reach {}\n\n{}", uri, e);
            start_polling(&ctx.proxy, target, actions);
            gateway_error(&msg, e, &ctx.config)
        }
    }
}

fn gateway_error(msg: &str, e: hyper::Error, config: &Config) -> Response<Body> {
    let html = PROXY_ERROR_HTML
        .replace("{{ error }}", msg)
        .replace("{{ reload_script }}", &inject::script(config));

    let status = if e.is_timeout() {
        StatusCode::GATEWAY_TIMEOUT
    } else {
        StatusCode::BAD_GATEWAY
    };

    Response::builder()
        .status(status)
        .header("Server", SERVER_HEADER)
        .header("Content-Type", "text/html")
        .body(html.into())
        .unwrap()
}

/// Regularly polls the proxy target until it is reachable again. Once it is, it
/// sends a reload action and stops. Makes sure (via `ctx`) that just one
/// polling instance exists per penguin server.
fn start_polling(ctx: &ProxyContext, target: &ProxyTarget, actions: Sender<Action>) {
    // We only need one task polling the target.
    let is_polling = Arc::clone(&ctx.is_polling_target);
    if is_polling.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        return;
    }

    let client = Client::builder().build::<_, hyper::Body>(HttpsConnector::new());
    let uri = Uri::builder()
        .scheme(target.scheme.clone())
        .authority(target.authority.clone())
        .path_and_query("/")
        .build()
        .unwrap();

    log::info!("Start regularly polling '{}' until it is available...", uri);
    tokio::spawn(async move {
        // We start polling quite quickly, but slow down up to this constant.
        const MAX_SLEEP_DURATION: Duration = Duration::from_secs(3);
        let mut sleep_duration = Duration::from_millis(250);

        loop {
            tokio::time::sleep(sleep_duration).await;
            sleep_duration = min(sleep_duration.mul_f32(1.5), MAX_SLEEP_DURATION);

            log::trace!("Trying to connect to '{}' again", uri);
            if client.get(uri.clone()).await.is_ok() {
                log::debug!("Reconnected to proxy target, reloading all active browser sessions");
                let _ = actions.send(Action::Reload);
                is_polling.store(false, Ordering::SeqCst);
                break;
            }
        }
    });
}
