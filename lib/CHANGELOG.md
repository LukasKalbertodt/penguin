# Changelog

All notable changes to the penguin **library** will be documented here.


## [Unreleased]

## [0.1.9] - 2025-07-15
- Fix `Content-Range` header for HTTP range requests

## [0.1.8] - 2023-11-26
- Add basic HTTP range request support for the file server. With this, video files served by Penguin can be played by Safari.
- Add body sniffing to detect HTML content (and insert reload script) more often (see #11)

## [0.1.7] - 2022-06-22
### Fixed
- Fix 404, "gateway error" and dir-listing pages that were broken in the previous release. (The JS code wasn't injected correctly, showing up as plain text. Woops.)

## [0.1.6] - 2022-06-08
### Fixed
- `Content-Security-Policy` (CSP) header is now potentially modified in proxy mode if required for penguin's injected script (`'self'` is potentially added to `script-src` and `connect-src`).

### Improved
- Update dependencies (this bumps the MSRV to 1.56!)

## [0.1.5] - 2022-04-19
### Added
- Add feature `vendored-openssl` to compile `openssl` from source
  [PR #10](https://github.com/LukasKalbertodt/penguin/pull/10) (Thanks @philipahlberg)

### Improved
- Updated dependencies

## [0.1.4] - 2021-10-02
### Improved
- Include reload script in 404 response: now the page can still reload itself
  after a 404 reply.
- After a getting a gateway error, automatically reload all browser sessions
  once the proxy is reachable again. This is done by regularly polling the
  proxy from the Penguin server.

### Fixed
- When using the proxy, the `host` HTTP-header is adjusted to the proxy target
  host (instead of the original `localhost:4090` that the browser sends).
- Correctly handle compression in proxy: gzip and brotli compression is
  supported and the HTTP body is decompressed before the reload script is
  injected. This was just totally broken before. The `accept-encoding` header
  of the request is also adjusted to not list anything but `gzip` and `br`.
- Rewrite `location` header to make HTTP redirects work with proxy.

## [0.1.3] - 2021-07-18
### Fixed
- Fix bug resulting in endless reloading if the proxy is slow
- Ignore one specific WS error that occurs often, is not important and caused
  lots of useless warnings
- Correctly handle ping messages (also getting rid of useless warnings)

## [0.1.2] - 2021-05-10
### Added
- All responses (except the ones forwarded from the proxy server) now contain
  the "server" HTTP header.

### Fixed
- Make Penguin work with non-`127.0.0.1` loopback addresses.
- Fix warning about directory traversal attack incorrectly being emitted.

## [0.1.1] - 2021-03-07
### Added
- `util::wait_for_proxy`

### Changed
- If the server cannot bind to the port, an error is returned from the server
  future instead of panicking.


## 0.1.0 - 2021-03-03
### Added
- Everything


[Unreleased]: https://github.com/LukasKalbertodt/penguin/compare/lib-v0.1.9...HEAD
[0.1.9]: https://github.com/LukasKalbertodt/penguin/compare/lib-v0.1.8...lib-v0.1.9
[0.1.8]: https://github.com/LukasKalbertodt/penguin/compare/lib-v0.1.7...lib-v0.1.8
[0.1.7]: https://github.com/LukasKalbertodt/penguin/compare/lib-v0.1.6...lib-v0.1.7
[0.1.6]: https://github.com/LukasKalbertodt/penguin/compare/lib-v0.1.5...lib-v0.1.6
[0.1.5]: https://github.com/LukasKalbertodt/penguin/compare/lib-v0.1.4...lib-v0.1.5
[0.1.4]: https://github.com/LukasKalbertodt/penguin/compare/lib-v0.1.3...lib-v0.1.4
[0.1.3]: https://github.com/LukasKalbertodt/penguin/compare/lib-v0.1.2...lib-v0.1.3
[0.1.2]: https://github.com/LukasKalbertodt/penguin/compare/lib-v0.1.1...lib-v0.1.2
[0.1.1]: https://github.com/LukasKalbertodt/penguin/compare/lib-v0.1.0...lib-v0.1.1
