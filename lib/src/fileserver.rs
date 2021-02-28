use std::{io::{self, ErrorKind}, path::Path};

use hyper::{Body, Request, Response};
use tokio::fs;
use tokio_util::codec::{FramedRead, BytesCodec};

use crate::{inject, Config, server::{bad_request, not_found}};


/// Checks if the request matches any `config.mounts` and returns an
/// appropriate response in that case. Otherwise `Ok(None)` is returned.
pub(crate) async fn try_serve(
    req: &Request<Body>,
    config: &Config,
) -> Option<Response<Body>> {
    let (subpath, sd) = config.mounts.iter()
        .filter_map(|sd| {
            req.uri()
                .path()
                .strip_prefix(&sd.uri_path)
                .map(|subpath| {
                    // Make sure that subpath never starts with `/`.
                    (subpath.trim_start_matches('/').to_owned(), sd)
                })
        })

        // We want the "most specific" mount, so the longest URI path wins.
        .max_by_key(|(_, sd)| sd.uri_path.len())?;

    Some(serve(req, &subpath, &sd.fs_path, config).await)
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

    if let Err(response) = check_directory_traversal_attack(&path, fs_root).await {
        log::warn!("Directory traversal attack detected -> responding BAD REQUEST");
        return response;
    }

    if !path.exists() {
        not_found()
    } else if path.is_file() {
        log::trace!("Serving requested file");
        serve_file(&path, config).await
    } else if path.join("index.html").is_file() {
        log::trace!("Serving 'index.html' file in requested directory");
        serve_file(&path.join("index.html"), config).await
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
        .replace("{{ reload_script }}", &inject::script(&config));

    Ok(
        Response::builder()
            .header("Content-Type", "text/html; charset=utf-8")
            .body(html.into())
            .expect("bug: invalid response")
    )
}

/// Serves a single file. If it's a HTML file, our JS code is injected.
async fn serve_file(path: &Path, config: &Config) -> Response<Body> {
    // TODO: maybe we should return 403 if the file can't be read due to
    // permissions?

    let mime = mime_guess::from_path(&path).first();
    if mime.as_ref().map_or(false, |mime| mime.as_ref().starts_with("text/html")) {
        let raw = fs::read(path).await.expect("failed to read file");
        let html = inject::into(&raw, &config);

        Response::builder()
            .header("Content-Type", "text/html")
            .header("Content-Length", html.len().to_string())
            .body(html.into())
            .expect("bug: invalid response")
    } else {
        let file = fs::File::open(path).await.expect("failed to open file");
        let body = Body::wrap_stream(FramedRead::new(file, BytesCodec::new()));

        let mut response = Response::builder();
        if let Some(mime) = mime {
            response = response.header("Content-Type", mime.to_string());
        }

        response.body(body).expect("bug: invalid response")
    }
}

/// Protects against directory traversal attacks
async fn check_directory_traversal_attack(path: &Path, fs_root: &Path) -> Result<(), Response<Body>> {
    fn map_error(e: io::Error) -> Response<Body> {
        if e.kind() == ErrorKind::NotFound {
            not_found()
        } else {
            panic!("could not canonicalize path");
        }
    }

    let canonical_req = fs::canonicalize(path).await.map_err(map_error)?;
    let canonical_root = fs::canonicalize(fs_root).await.map_err(map_error)?;

    if !canonical_req.starts_with(canonical_root) {
        return Err(bad_request("Bad request: requested file outside of served directory\n"));
    }

    Ok(())
}
