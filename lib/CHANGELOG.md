# Changelog

All notable changes to the penguin **library** will be documented here.


## [Unreleased]

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


[Unreleased]: https://github.com/LukasKalbertodt/penguin/compare/lib-v0.1.4...HEAD
[0.1.4]: https://github.com/LukasKalbertodt/penguin/compare/lib-v0.1.3...lib-v0.1.4
[0.1.3]: https://github.com/LukasKalbertodt/penguin/compare/lib-v0.1.2...lib-v0.1.3
[0.1.2]: https://github.com/LukasKalbertodt/penguin/compare/lib-v0.1.1...lib-v0.1.2
[0.1.1]: https://github.com/LukasKalbertodt/penguin/compare/lib-v0.1.0...lib-v0.1.1
