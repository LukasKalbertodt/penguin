use std::str::FromStr;

use hyper::http::uri::{Authority, Scheme};

use super::*;

/// Asserts that certain traits are implemented for the public types. `Debug` is
/// already covered by `#![deny(missing_debug_implementations)]`.
#[allow(unused_variables, dead_code, unreachable_code)]
mod traits {
    use super::*;

    fn controller() -> impl Send + Sync + Clone + Unpin {
        let x: Controller = todo!();
        x
    }

    fn server() -> impl Send + Unpin {
        let x: Server = todo!();
        x
    }
}


#[test]
fn parse_proxy_target() {
    assert_eq!(
        ProxyTarget::from_str("localhost").unwrap(),
        ProxyTarget::from((Scheme::HTTP, Authority::from_static("localhost"))),
    );
    assert_eq!(
        ProxyTarget::from_str("localhost:8000").unwrap(),
        ProxyTarget::from((Scheme::HTTP, Authority::from_static("localhost:8000"))),
    );
    assert_eq!(
        ProxyTarget::from_str("https://127.0.0.1:30").unwrap(),
        ProxyTarget::from((Scheme::HTTPS, Authority::from_static("127.0.0.1:30"))),
    );
    assert_eq!(
        ProxyTarget::from_str("http://github.com").unwrap(),
        ProxyTarget::from((Scheme::HTTP, Authority::from_static("github.com"))),
    );
    assert_eq!(
        ProxyTarget::from_str("https://github.com/").unwrap(),
        ProxyTarget::from((Scheme::HTTPS, Authority::from_static("github.com"))),
    );
}

#[test]
fn parse_proxy_target_bad() {
    assert!(matches!(
        ProxyTarget::from_str("").unwrap_err(),
        ProxyTargetParseError::InvalidUri(_),
    ));
    assert!(matches!(
        ProxyTarget::from_str("github.com").unwrap_err(),
        ProxyTargetParseError::MissingScheme,
    ));
    assert!(matches!(
        ProxyTarget::from_str("https://").unwrap_err(),
        ProxyTargetParseError::InvalidUri(_),
    ));
    assert!(matches!(
        ProxyTarget::from_str("http://github.com/foo").unwrap_err(),
        ProxyTargetParseError::HasPath,
    ));
}
