use std::{net::SocketAddr, path::PathBuf, str::FromStr};

use hyper::{Uri, http::uri};


/// The URI path which is used for penguin internal control functions (e.g.
/// opening WS connections).
///
/// We need a path that:
/// - is unlikely to clash with real paths of existing web applications,
/// - is still somewhat easy to type and remember (e.g. to send requests via
///   `curl`), and
/// - doesn't use any invalid or unsafe characters for URLs.
///
/// For URI paths, the characters a-z, A-Z, 0-9 and `- . _ ~` are "safe": they
/// don't need to be escaped and don't have a special meaning.
const DEFAULT_CONTROL_PATH: &str = "/~~penguin";

/// Configuration for the penguin server. This type uses the builder pattern to
#[derive(Debug)]
pub struct Config {
    /// The port/socket address the server should be listening on.
    pub(crate) bind_addr: SocketAddr,

    /// Proxy target that HTTP requests should be forwarded to.
    pub(crate) proxy: Option<ProxyTarget>,

    /// A list of directories to serve as a file server. As expected from other
    /// file servers, this lists the contents of directories and serves files
    /// directly. HTML files are injected with the penguin JS code.
    pub(crate) serve_dirs: Vec<ServeDir>,

    /// HTTP requests to this path are interpreted by this library to perform
    /// its function and are not normally served via the reverse proxy or the
    /// static file server.
    ///
    /// Has to start with `/` and *not* include the trailing `/`.
    ///
    /// TODO: maybe allow using a separate port instead of a control path.
    pub(crate) control_path: String,

    // TODO:
    // - callback
    // - string/name of service ("floof") for error pages
}

impl Config {
    /// Creates a new configuration. The `bind_addr` is what the server will
    /// listen on.
    pub fn new(bind_addr: SocketAddr) -> Self {
        Self {
            bind_addr,
            proxy: None,
            control_path: DEFAULT_CONTROL_PATH.into(),
            serve_dirs: Vec::new(),
        }
    }

    /// Enables proxying request to the given proxy target.
    pub fn proxy(mut self, target: ProxyTarget) -> Self {
        self.proxy = Some(target);
        self
    }

    /// Adds one directory to be served under `uri_path`.
    ///
    /// The following conditions of `uri_path` need to be fulfilled or otherwise
    /// this function will panic:
    ///
    /// - `uri_path` must start with `/`.
    /// - `uri_path` must not end in `/`.
    /// - `uri_path` must not share a prefix with any previously added
    ///   `uri_path`.
    ///
    ///
    /// TODO: add example
    pub fn add_serve_dir(
        mut self,
        uri_path: impl Into<String>,
        fs_path: impl Into<PathBuf>,
    ) -> Self {
        let uri_path = uri_path.into();

        // Check validity of URI path.
        assert!(uri_path.starts_with('/'));
        assert!(uri_path == "/" || !uri_path.ends_with('/'));
        for other in &self.serve_dirs {
            if other.uri_path.starts_with(&uri_path) || uri_path.starts_with(&other.uri_path) {
                panic!(
                    "URI path '{}' shares a prefix with previous URI path '{}'",
                    uri_path,
                    other.uri_path,
                );
            }
        }

        self.serve_dirs.push(ServeDir {
            uri_path,
            fs_path: fs_path.into(),
        });
        self
    }
}

#[derive(Debug)]
pub struct ProxyTarget {
    pub(crate) scheme: uri::Scheme,
    pub(crate) authority: uri::Authority,
}

impl From<(uri::Scheme, uri::Authority)> for ProxyTarget {
    fn from((scheme, authority): (uri::Scheme, uri::Authority)) -> Self {
        Self { scheme, authority }
    }
}

impl FromStr for ProxyTarget {
    type Err = ProxyTargetError;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let parts = src.parse::<Uri>()?.into_parts();
        let has_real_path = parts.path_and_query.as_ref()
            .map_or(false, |pq| !pq.as_str().is_empty() && pq.as_str() != "/");
        if has_real_path {
            return Err(ProxyTargetError::HasPath);
        }

        Ok(Self {
            scheme: parts.scheme.ok_or(ProxyTargetError::MissingScheme)?,
            authority: parts.authority.ok_or(ProxyTargetError::MissingAuthority)?,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProxyTargetError {
    #[error("invalid URI: {0}")]
    InvalidUri(#[from] uri::InvalidUri),

    #[error("proxy target has path which is not allowed")]
    HasPath,

    #[error("proxy target has no scheme ('http' or 'https') specified")]
    MissingScheme,

    #[error("proxy target has no authority (\"host\") specified")]
    MissingAuthority,
}

/// A mapping from URI path to file system path.
#[derive(Debug)]
pub(crate) struct ServeDir {
    /// Path prefix of the URI that will map to the directory. Has to start with
    /// `/` and *not* include the trailing `/`.
    pub(crate) uri_path: String,

    /// Path to a directory on the file system that is served under the
    /// specified URI path.
    pub(crate) fs_path: PathBuf,
}
