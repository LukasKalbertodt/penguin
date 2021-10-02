use std::{
    cmp::min,
    convert::{TryFrom, TryInto},
    io::Read,
    sync::{Arc, atomic::{AtomicBool, Ordering}},
    time::Duration,
};

use hyper::{
    Body, Client, Request, Response, StatusCode, Uri,
    body::Bytes,
    header::{self, HeaderValue},
    http::uri::Scheme,
};
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
    adjust_request(&mut req, target);
    let uri = req.uri().clone();

    log::trace!("Forwarding request to proxy target {}", uri);
    let client = Client::builder().build::<_, hyper::Body>(HttpsConnector::new());
    match client.request(req).await {
        Ok(response) => adjust_response(response, ctx, &uri, target, &ctx.config).await,
        Err(e) => {
            log::warn!("Failed to reach proxy target '{}': {}", uri, e);
            let msg = format!("Failed to reach {}\n\n{}", uri, e);
            start_polling(&ctx.proxy, target, actions);
            gateway_error(&msg, e, &ctx.config)
        }
    }
}

fn adjust_request(req: &mut Request<Body>, target: &ProxyTarget) {
    // Change the URI to the proxy target.
    let uri = {
        let mut parts = req.uri().clone().into_parts();
        parts.scheme = Some(target.scheme.clone());
        parts.authority = Some(target.authority.clone());
        Uri::from_parts(parts).expect("bug: invalid URI")
    };
    *req.uri_mut() = uri.clone();

    // If the `host` header is set, we need to adjust it, too.
    if let Some(host) = req.headers_mut().get_mut(header::HOST) {
        // `http::Uri` already does not parse non-ASCII hosts. Unicode hosts
        // have to be encoded as punycode.
        *host = HeaderValue::from_str(target.authority.as_str())
            .expect("bug: URI authority should be ASCII");
    }

    // Deal with compression.
    if let Some(header) = req.headers_mut().get_mut(header::ACCEPT_ENCODING) {
        // In a production product, panicking here is not OK. But all encodings
        // listed in [1] and the syntax described in [2] only contain ASCII
        // bytes. So non-ASCII bytes here are highly unlikely.
        //
        // [1]: https://www.iana.org/assignments/http-parameters/http-parameters.xml
        // [2]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Accept-Encoding
        let value = header.to_str()
            .expect("'accept-encoding' header value contains non-ASCII bytes");
        let new_value = filter_encodings(&value);

        if new_value.is_empty() {
            req.headers_mut().remove(header::ACCEPT_ENCODING);
        } else {
            // It was ASCII before and we do not add any non-ASCII values.
            *header = HeaderValue::try_from(new_value)
                .expect("bug: non-ASCII values in new 'accept-encoding' header");
        }
    }
}

/// We support only gzip and brotli. But according to this statistics, those two
/// make up the vast majority of requests:
/// https://almanac.httparchive.org/en/2019/compression
const SUPPORTED_COMPRESSIONS: &[&str] = &["gzip", "br", "identity"];

async fn adjust_response(
    mut response: Response<Body>,
    ctx: &Context,
    uri: &Uri,
    target: &ProxyTarget,
    config: &Config,
) -> Response<Body> {
    // Rewrite `location` header if it's present.
    if let Some(header) = response.headers_mut().get_mut(header::LOCATION) {
        rewrite_location(header, target, config);
    }

    let content_type = response.headers().get(header::CONTENT_TYPE);
    let is_html = content_type.map_or(false, |v| v.as_ref().starts_with(b"text/html"));
    if !is_html {
        return response;
    }

    log::trace!("Response from proxy is HTML: injecting script");

    // The response is HTML: we need to download it completely and
    // inject our script.
    let (mut parts, body) = response.into_parts();
    let body = match hyper::body::to_bytes(body).await {
        Ok(body) => body,
        Err(e) => {
            log::warn!("Failed to download full response from proxy target");
            let msg = format!("Failed to download response from {}\n\n{}", uri, e);
            return gateway_error(&msg, e, &ctx.config);
        }
    };

    // Uncompress if necessary. All this allocates more than necessary, but I'd
    // rather keep easier code in this case, as performance is unlikely to
    // matter.
    let new_body = match parts.headers.get(header::CONTENT_ENCODING).map(|v| v.as_bytes()) {
        None => Bytes::from(inject::into(&body, &ctx.config)),

        Some(b"gzip") => {
            let mut decompressed = Vec::new();
            flate2::read::GzDecoder::new(&*body).read_to_end(&mut decompressed)
                .expect("unexpected error while decompressing GZIP");
            let injected = inject::into(&decompressed, &ctx.config);
            let mut out = Vec::new();
            flate2::read::GzEncoder::new(&*injected, flate2::Compression::best())
                .read_to_end(&mut out)
                .expect("unexpected error while compressing GZIP");
            Bytes::from(out)
        }

        Some(b"br") => {
            let mut decompressed = Vec::new();
            brotli::BrotliDecompress(&mut &*body, &mut decompressed)
                .expect("unexpected error while decompressing Brotli");
            let injected = inject::into(&decompressed, &ctx.config);
            let mut out = Vec::new();
            brotli::BrotliCompress(&mut &*injected, &mut out, &Default::default())
                .expect("unexpected error while compressing Brotli");
            Bytes::from(out)
        }

        Some(other) => {
            log::warn!(
                "Unsupported content encoding '{}'. Not injecting script!",
                String::from_utf8_lossy(other),
            );
            body
        }
    };

    if let Some(content_len) = parts.headers.get_mut(header::CONTENT_LENGTH) {
        *content_len = new_body.len().into();
    }
    Response::from_parts(parts, new_body.into())
}

fn rewrite_location(header: &mut HeaderValue, target: &ProxyTarget, config: &Config) {
    let value = match std::str::from_utf8(header.as_bytes()) {
        Err(_) => {
            log::warn!("Non UTF-8 'location' header: not rewriting");
            return;
        }
        Ok(v) => v,
    };

    let mut uri = match value.parse::<Uri>() {
        Err(_) => {
            log::warn!("Could not parse 'location' header as URI: not rewriting");
            return;
        }
        Ok(uri) => uri.into_parts(),
    };

    // If the redirect points to the proxy target itself (i.e. an internal
    // redirect), we change the `location` header so that the browser changes
    // the path & query, but stays on the Penguin host.
    if uri.authority.as_ref() == Some(&target.authority) {
        // Penguin itself only listens on HTTP
        uri.scheme = Some(Scheme::HTTP);
        let authority = config.bind_addr.to_string()
            .try_into()
            .expect("bind addr is not a valid authority");
        uri.authority = Some(authority);

        let uri = Uri::from_parts(uri).expect("bug: failed to build URI");
        *header = HeaderValue::from_bytes(uri.to_string().as_bytes())
            .expect("bug: new 'location' is invalid header value");
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

/// Filter the "accept-encoding" encodings in the header value `orig` and return
/// a new value only containing the ones we support.
fn filter_encodings(orig: &str) -> String {
    let allowed_values = orig.split(',')
        .map(|part| part.trim())
        .filter(|part| {
            let encoding = part.split_once(';').map(|p| p.0).unwrap_or(part);
            SUPPORTED_COMPRESSIONS.contains(&encoding)
        });

    let mut new_value = String::new();
    for (i, part) in allowed_values.enumerate() {
        if i != 0 {
            new_value.push_str(", ");
        }
        new_value.push_str(part);
    }
    new_value
}


#[cfg(test)]
mod tests {
    #[test]
    fn encoding_filter() {
        use super::filter_encodings as filter;

        assert_eq!(filter(""), "");
        assert_eq!(filter("gzip"), "gzip");
        assert_eq!(filter("br"), "br");
        assert_eq!(filter("gzip, br"), "gzip, br");
        assert_eq!(filter("gzip, deflate"), "gzip");
        assert_eq!(filter("deflate, gzip"), "gzip");
        assert_eq!(filter("gzip, deflate, br"), "gzip, br");
        assert_eq!(filter("deflate, gzip, br"), "gzip, br");
        assert_eq!(filter("gzip, br, deflate"), "gzip, br");
        assert_eq!(filter("deflate"), "");
        assert_eq!(filter("br;q=1.0, deflate;q=0.5, gzip;q=0.8, *;q=0.1"), "br;q=1.0, gzip;q=0.8");
    }
}
