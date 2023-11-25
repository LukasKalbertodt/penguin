use std::{io::{self, ErrorKind}, path::Path};

use http_range::{HttpRange, HttpRangeParseError};
use hyper::{Body, Request, Response, header, StatusCode};
use tokio::{fs, io::{AsyncSeekExt, AsyncReadExt}};
use tokio_util::codec::{FramedRead, BytesCodec};

use crate::{inject, Config};
use super::{bad_request, not_found, SERVER_HEADER};


/// Checks if the request matches any `config.mounts` and returns an
/// appropriate response in that case. Otherwise `Ok(None)` is returned.
pub(crate) async fn try_serve(
    req: &Request<Body>,
    config: &Config,
) -> Option<Response<Body>> {
    let (subpath, mount) = config.mounts.iter()
        .filter_map(|mount| {
            req.uri()
                .path()
                .strip_prefix(&mount.uri_path)
                .map(|subpath| {
                    // Make sure that subpath never starts with `/`.
                    (subpath.trim_start_matches('/').to_owned(), mount)
                })
        })

        // We want the "most specific" mount, so the longest URI path wins.
        .max_by_key(|(_, mount)| mount.uri_path.len())?;

    Some(serve(req, &subpath, &mount.fs_path, config).await)
}

async fn serve(
    req: &Request<Body>,
    subpath: &str,
    fs_root: &Path,
    config: &Config,
) -> Response<Body> {
    log::trace!("Serving request from file server...");

    let subpath = Path::new(subpath);
    let path = fs_root.join(subpath);

    // Protect against directory traversal attacks.
    macro_rules! canonicalize {
        ($path:expr) => {
            match fs::canonicalize($path).await {
                Ok(v) => v,
                Err(e) if e.kind() == ErrorKind::NotFound => return not_found(config),
                Err(e) => panic!(
                    "unhandled error: could not canonicalize path '{}': {}",
                    $path.display(),
                    e,
                ),
            }
        };
    }

    let canonical_req = canonicalize!(&path);
    let canonical_root = canonicalize!(fs_root);
    if !canonical_req.starts_with(canonical_root) {
        log::warn!(
            "Directory traversal attack detected ({:?} {}) -> responding BAD REQUEST",
            req.method(),
            req.uri().path(),
        );

        return bad_request("Bad request: requested file outside of served directory\n");
    }

    // Dispatch depending on whether it's a file or directory.
    if !path.exists() {
        not_found(config)
    } else if path.is_file() {
        log::trace!("Serving requested file");
        serve_file(&path, req, config).await
    } else if path.join("index.html").is_file() {
        log::trace!("Serving 'index.html' file in requested directory");
        serve_file(&path.join("index.html"), req, config).await
    } else {
        log::trace!("Listing contents of directory...");
        serve_dir(req.uri().path(), &path, config)
            .await
            .expect("failed to read directory contents due to IO error")
    }
}

/// Lists the contents of a directory.
async fn serve_dir(
    uri_path: &str,
    path: &Path,
    config: &Config,
) -> Result<Response<Body>, io::Error> {
    const DIR_LISTING_HTML: &str = include_str!("../assets/dir-listing.html");

    // Collect all children of this folder.
    let mut folders = Vec::new();
    let mut files = Vec::new();
    let mut it = fs::read_dir(path).await?;
    while let Some(entry) = it.next_entry().await? {
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type().await?.is_file() {
            files.push((name, false));
        } else {
            folders.push((name + "/", false));
        }
    }

    // Also collect all mounts that are mounted below this path.
    for sd in config.mounts.iter().filter(|sd| sd.fs_path.exists()) {
        if let Some(rest) = sd.uri_path.strip_prefix(uri_path) {
            if rest.is_empty() {
                continue;
            }

            let name = rest.find('/')
                .map(|pos| &rest[..pos])
                .unwrap_or(rest)
                .to_owned();
            if sd.fs_path.is_dir() {
                folders.push((name + "/", true));
            } else {
                files.push((name, true));
            }
        }
    }

    folders.sort();
    files.sort();

    // Build list of children.
    let mut entries = String::from("\n");
    for (name, is_mount) in folders.into_iter().chain(files) {
        entries.push_str(&format!(
            "<li><a href=\"{0}\" class=\"{1}\"><code>{0}</code></a></li>\n",
            name,
            if is_mount { "mount" } else { "real" },
        ));
    }

    let html = DIR_LISTING_HTML
        .replace("{{ uri_path }}", uri_path)
        .replace("{{ entries }}", &entries)
        .replace("{{ control_path }}", config.control_path());

    Ok(
        Response::builder()
            .header("Content-Type", "text/html; charset=utf-8")
            .header("Server", SERVER_HEADER)
            .body(html.into())
            .expect("bug: invalid response")
    )
}

/// Serves a single file. If it's a HTML file, our JS code is injected.
async fn serve_file(
    path: &Path,
    req: &Request<Body>,
    config: &Config,
) -> Response<Body> {
    // TODO: maybe we should return 403 if the file can't be read due to
    // permissions? Generally, the `unwrap`s in this function are... meh.

    let mime = mime_guess::from_path(&path).first();
    if mime.as_ref().map_or(false, |mime| mime.as_ref().starts_with("text/html")) {
        let raw = fs::read(path).await.expect("failed to read file");
        let html = inject::into(&raw, &config);

        Response::builder()
            .header("Content-Type", "text/html")
            .header("Content-Length", html.len().to_string())
            .header("Server", SERVER_HEADER)
            .body(html.into())
            .expect("bug: invalid response")
    } else {
        let mut file = fs::File::open(path).await.expect("failed to open file");
        let file_size = file.metadata().await.expect("failed to read file metadata").len();

        let mut response = Response::builder()
            .header("Server", SERVER_HEADER)
            .header(header::ACCEPT_RANGES, "bytes");
        if let Some(mime) = mime {
            response = response.header("Content-Type", mime.to_string());
        }

        if let Some(range_header) = req.headers().get(header::RANGE) {
            let range = match HttpRange::parse_bytes(range_header.as_bytes(), file_size) {
                Ok(ranges) if ranges.len() == 1 => ranges[0],
                Ok(_) => {
                    return Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .header("Server", SERVER_HEADER)
                        .body("multiple ranges in 'Range' header not supported".into())
                        .expect("bug: invalid response")
                }
                Err(HttpRangeParseError::InvalidRange) => todo!(),
                Err(HttpRangeParseError::NoOverlap) => {
                    return Response::builder()
                        .status(StatusCode::RANGE_NOT_SATISFIABLE)
                        .header("Server", SERVER_HEADER)
                        .body("".into())
                        .expect("bug: invalid response");
                }
            };

            file.seek(io::SeekFrom::Start(range.start)).await.unwrap();
            let reader = FramedRead::new(file.take(range.length), BytesCodec::new());
            let body = Body::wrap_stream(reader);
            response
                .status(StatusCode::PARTIAL_CONTENT)
                .header(header::CONTENT_LENGTH, range.length)
                .header(header::CONTENT_RANGE, format!(
                    "bytes {}-{}/{}",
                    range.start,
                    range.start + range.length,
                    file_size,
                ))
                .body(body)
                .expect("bug: invalid response")
        } else {
            let body = Body::wrap_stream(FramedRead::new(file, BytesCodec::new()));
            response
                .header(header::CONTENT_LENGTH, file_size)
                .body(body)
                .expect("bug: invalid response")
        }
    }
}
