use std::{path::Path, sync::Arc};

use hyper::{Body, Request, Response, StatusCode};
use tokio::fs;
use tokio_util::codec::{FramedRead, BytesCodec};

use crate::{inject, Config, Error};


/// Checks if the request matches any `config.serve_dirs` and returns an
/// appropriate response in that case. Otherwise `Ok(None)` is returned.
pub(crate) async fn try_serve(
    req: Request<Body>,
    config: Arc<Config>,
) -> Result<Option<Response<Body>>, Error> {
    let dir = config.serve_dirs.iter().find_map(|sd| {
        req.uri()
            .path()
            .strip_prefix(&sd.uri_path)
            .map(|subpath| (subpath.to_owned(), &sd.fs_path))
    });

    match dir {
        Some((subpath, fs_root)) => serve(req, &subpath, fs_root, &config).await.map(Some),
        None => Ok(None),
    }
}

async fn serve(
    req: Request<Body>,
    subpath: &str,
    fs_root: &Path,
    config: &Config,
) -> Result<Response<Body>, Error> {
    let subpath = Path::new(subpath);
    let path = fs_root.join(subpath);

    // Check that the resulting file we serve is actually a child of the root
    // directory, i.e. protect against path traversal attacks.
    if !fs::canonicalize(&path).await?.starts_with(fs::canonicalize(fs_root).await?) {
        let bad_req = Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from("Bad request: requested file outside of served directory"))
            .expect("bug: invalid response");

        return Ok(bad_req);
    }

    let response = if !path.exists() {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not found"))
            .expect("bug: invalid response")
    } else if path.is_file() {
        serve_file(&path, config).await?
    } else {
        serve_dir(req.uri().path(), &path, config).await?
    };

    Ok(response)
}

/// Lists the contents of a directory.
async fn serve_dir(uri_path: &str, path: &Path, config: &Config) -> Result<Response<Body>, Error> {
    const DIR_LISTING_HTML: &str = include_str!("assets/dir-listing.html");

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
    for sd in config.serve_dirs.iter().filter(|sd| sd.fs_path.exists()) {
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
    let mut entries = String::new();
    for (name, is_mount) in folders.into_iter().chain(files) {
        entries.push_str(&format!(
            r#"<li><a href="{0}" class="{1}"><code>{0}</code></a></li>"#,
            name,
            if is_mount { "mount" } else { "real" },
        ));
    }

    let html = DIR_LISTING_HTML
        .replace("{{ uri_path }}", uri_path)
        .replace("{{ entries }}", &entries)
        .replace("{{ reload_script }}", &inject::script(&config));

    Ok(
        Response::builder()
            .header("Content-Type", "text/html; charset=utf-8")
            .body(html.into())
            .expect("bug: invalid response")
    )
}

/// Serves a single file. If it's a HTML file, our JS code is injected.
async fn serve_file(path: &Path, config: &Config) -> Result<Response<Body>, Error> {
    let mime = mime_guess::from_path(&path).first();
    if mime.as_ref().map_or(false, |mime| mime.as_ref().starts_with("text/html")) {
        let raw = fs::read(path).await?;
        let html = inject::into(&raw, &config);

        Ok(
            Response::builder()
                .header("Content-Type", "text/html")
                .header("Content-Length", html.len().to_string())
                .body(html.into())
                .expect("bug: invalid response")
        )
    } else {
        let file = fs::File::open(path).await?;
        let body = Body::wrap_stream(FramedRead::new(file, BytesCodec::new()));

        let mut response = Response::builder();
        if let Some(mime) = mime {
            response = response.header("Content-Type", mime.to_string());
        }

        Ok(response.body(body).expect("bug: invalid response"))
    }
}
