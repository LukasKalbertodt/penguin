use std::{fmt, net::{IpAddr, SocketAddr}, path::PathBuf, str::FromStr};

use hyper::{Uri, http::uri};

use crate::{Controller, Server};


/// The URI path which is used for penguin internal control functions (e.g.
/// opening WS connections).
///
/// We need a path that:
/// - is unlikely to clash with real paths of existing web applications,
/// - is still somewhat easy to type and remember (e.g. to send requests via
///   `curl`), and
/// - doesn't use any invalid characters for URLs.
pub const DEFAULT_CONTROL_PATH: &str = "/~~penguin";

/// A valid penguin server configuration.
///
/// To create a configuration, use [`Server::bind`] to obtain a [`Builder`]
/// which can be turned into a `Config`.
#[derive(Debug, Clone)]
pub struct Config {
    /// The port/socket address the server should be listening on.
    pub(crate) bind_addr: SocketAddr,

    /// Proxy target that HTTP requests should be forwarded to.
    pub(crate) proxy: Option<ProxyTarget>,

    /// A list of directories to serve as a file server. As expected from other
    /// file servers, this lists the contents of directories and serves files
    /// directly. HTML files are injected with the penguin JS code.
    pub(crate) mounts: Vec<Mount>,

    /// HTTP requests to this path are interpreted by this library to perform
    /// its function and are not normally served via the reverse proxy or the
    /// static file server.
    ///
    /// Has to start with `/` and *not* include the trailing `/`.
    pub(crate) control_path: String,
}

impl Config {
    pub fn proxy(&self) -> Option<&ProxyTarget> {
        self.proxy.as_ref()
    }

    pub fn mounts(&self) -> &[Mount] {
        &self.mounts
    }

    pub fn control_path(&self) -> &str {
        &self.control_path
    }
}

/// Builder for the configuration of `Server`.
#[derive(Debug, Clone)]
pub struct Builder(Config);

impl Builder {
    /// Creates a new configuration. The `bind_addr` is what the server will
    /// listen on.
    pub(crate) fn new(bind_addr: SocketAddr) -> Self {
        Self(Config {
            bind_addr,
            proxy: None,
            control_path: DEFAULT_CONTROL_PATH.into(),
            mounts: Vec::new(),
        })
    }

    /// Enables and sets a proxy: incoming requests (that do not match a mount)
    /// are forwarded to the given proxy target and its response is forwarded
    /// back to the initiator of the request.
    ///
    /// **Panics** if this method is called more than once on a single
    /// `Builder`.
    pub fn proxy(mut self, target: ProxyTarget) -> Self {
        if let Some(prev) = self.0.proxy {
            panic!(
                "`Builder::proxy` called a second time: is called with '{}' now \
                    but was previously called with '{}'",
                target,
                prev,
            );
        }

        self.0.proxy = Some(target);
        self
    }

    /// Adds a mount: a directory to be served via file server under `uri_path`.
    /// The order in which the serve dirs are added does not matter. When
    /// serving a request, the most specific matching entry "wins".
    ///
    /// This method returns `ConfigError::DuplicateUriPath` if the same
    /// `uri_path` was added before.
    pub fn add_mount(
        mut self,
        uri_path: impl Into<String>,
        fs_path: impl Into<PathBuf>,
    ) -> Result<Self, ConfigError> {
        let mut uri_path = uri_path.into();
        normalize_path(&mut uri_path);

        if self.0.mounts.iter().any(|other| other.uri_path == uri_path) {
            return Err(ConfigError::DuplicateUriPath(uri_path));
        }

        self.0.mounts.push(Mount {
            uri_path,
            fs_path: fs_path.into(),
        });

        Ok(self)
    }

    /// Overrides the control path (`/~~penguin` by default) with a custom path.
    ///
    /// This is only useful if your web application wants to use the route
    /// `/~~penguin`.
    pub fn set_control_path(mut self, path: impl Into<String>) -> Self {
        self.0.control_path = path.into();
        normalize_path(&mut self.0.control_path);
        self
    }

    /// Validates the configuration and builds the server and controller from
    /// it. This is a shortcut for [`Builder::validate`] plus [`Server::build`].
    pub fn build(self) -> Result<(Server, Controller), ConfigError> {
        self.validate().map(Server::build)
    }

    /// Validates the configuration and returns the finished [`Config`].
    pub fn validate(self) -> Result<Config, ConfigError> {
        if self.0.proxy.is_none() && self.0.mounts.is_empty() {
            return Err(ConfigError::NoProxyOrMount)
        }

        if self.0.proxy.is_some() && self.0.mounts.iter().any(|other| other.uri_path == "/") {
            return Err(ConfigError::ProxyAndRootMount);
        }

        Ok(self.0)
    }
}

fn normalize_path(path: &mut String) {
    if path.len() > 1 && path.ends_with('/') {
        path.pop();
    }
    if !path.starts_with('/') {
        path.insert(0, '/');
    }
}

/// Configuration validation error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ConfigError {
    #[error("URI path '{0}' was added as mount twice")]
    DuplicateUriPath(String),

    #[error("a proxy was configured but a mount on '/' was added as well (in \
        that case, the proxy is would be ignored)")]
    ProxyAndRootMount,

    #[error("neither a proxy nor a mount was specified: server would always \
        respond 404 in this case")]
    NoProxyOrMount,
}

/// Defintion of a proxy target consisting of a scheme and authority (≈host).
///
/// To create this type you can:
/// - use the `FromStr` impl: `"http://localhost:8000".parse()`, or
/// - use the `From<(Scheme, Authority)>` impl.
///
/// The `FromStr` allows omitting the scheme ('http' or 'https') if the host is
/// `"localhost"` or a loopback address and defaults to 'http' in that case. For
/// all other hosts, the scheme has to be specified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyTarget {
    pub(crate) scheme: uri::Scheme,
    pub(crate) authority: uri::Authority,
}

impl From<(uri::Scheme, uri::Authority)> for ProxyTarget {
    fn from((scheme, authority): (uri::Scheme, uri::Authority)) -> Self {
        Self { scheme, authority }
    }
}

impl fmt::Display for ProxyTarget {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}://{}", self.scheme, self.authority)
    }
}

impl FromStr for ProxyTarget {
    type Err = ProxyTargetParseError;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let parts = src.parse::<Uri>()?.into_parts();
        let has_real_path = parts.path_and_query.as_ref()
            .map_or(false, |pq| !pq.as_str().is_empty() && pq.as_str() != "/");
        if has_real_path {
            return Err(ProxyTargetParseError::HasPath);
        }

        let authority = parts.authority.ok_or(ProxyTargetParseError::MissingAuthority)?;
        let scheme = parts.scheme
            .or_else(|| {
                // If the authority is a loopback IP or "localhost", we default to HTTP as scheme.
                let ip = authority.host().parse::<IpAddr>();
                if authority.host() == "localhost" || ip.map_or(false, |ip| ip.is_loopback()) {
                    Some(uri::Scheme::HTTP)
                } else {
                    None
                }
            })
            .ok_or(ProxyTargetParseError::MissingScheme)?;

        Ok(Self { scheme, authority })
    }
}

/// Error that can occur when parsing a `ProxyTarget` from a string.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ProxyTargetParseError {
    /// The string could not be parsed as `http::Uri`.
    #[error("invalid URI: {0}")]
    InvalidUri(#[from] uri::InvalidUri),

    /// The parsed URL has a path, but a proxy target must not have a path.
    #[error("proxy target has path which is not allowed")]
    HasPath,

    /// The URI does not have a scheme ('http' or 'https') specified when it
    /// should have.
    #[error("proxy target has no scheme ('http' or 'https') specified, but a \
        scheme must be specified for non-local targets")]
    MissingScheme,

    /// The URI does not have an authority (≈ "host"), but it needs one.
    #[error("proxy target has no authority (\"host\") specified")]
    MissingAuthority,
}

/// A mapping from URI path to file system path.
#[derive(Debug, Clone)]
pub struct Mount {
    /// Path prefix of the URI that will map to the directory. Has to start with
    /// `/` and *not* include the trailing `/`.
    pub uri_path: String,

    /// Path to a directory on the file system that is served under the
    /// specified URI path.
    pub fs_path: PathBuf,
}
