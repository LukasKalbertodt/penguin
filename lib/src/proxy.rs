use std::sync::Arc;

use hyper::{Body, Client, Request, Response, StatusCode, Uri, header};

use crate::{inject, Config, Error, ProxyTarget};


/// HTML content to reply in case an error occurs when connecting to the proxy.
const PROXY_ERROR_HTML: &str = include_str!("assets/proxy-error.html");

/// Forwards the given request to the specified proxy target and returns its
/// response.
///
/// If the proxy target cannot be reached, a 502 Bad Gateway or 504 Gateway
/// Timeout response is returned.
pub(crate) async fn forward(
    mut req: Request<Body>,
    target: &ProxyTarget,
    config: Arc<Config>,
) -> Result<Response<Body>, Error> {
    // Build new URI and change the given request.
    let uri = {
        let mut parts = req.uri().clone().into_parts();
        parts.scheme = Some(target.scheme.clone());
        parts.authority = Some(target.authority.clone());
        Uri::from_parts(parts).expect("bug: invalid URI")
    };
    *req.uri_mut() = uri.clone();

    let response = match Client::new().request(req).await {
        Ok(response) => {
            let content_type = response.headers().get(header::CONTENT_TYPE);
            if content_type.map_or(false, |v| v.as_ref().starts_with(b"text/html")) {
                // The response is HTML: we need to download it completely and
                // inject our script.
                let (parts, body) = response.into_parts();
                let body = hyper::body::to_bytes(body).await?;

                let new_body = inject::into(&body, &config);
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
            let msg = format!("Failed to reach {}\n\n{}", uri, e);
            let html = PROXY_ERROR_HTML
                .replace("{{ error }}", &msg)
                .replace("{{ reload_script }}", &inject::script(&config));

            let status = if e.is_timeout() {
                StatusCode::GATEWAY_TIMEOUT
            } else {
                StatusCode::BAD_GATEWAY
            };

            Response::builder()
                .status(status)
                .header("Content-Type", "text/html")
                .body(html.into())
                .unwrap()
        }
    };

    Ok(response)
}
