# Changelog

All notable changes to the penguin **library** will be documented here.


## [Unreleased]

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


[Unreleased]: https://github.com/LukasKalbertodt/penguin/compare/lib-v0.1.3...HEAD
[0.1.3]: https://github.com/LukasKalbertodt/penguin/compare/lib-v0.1.2...lib-v0.1.3
[0.1.2]: https://github.com/LukasKalbertodt/penguin/compare/lib-v0.1.1...lib-v0.1.2
[0.1.1]: https://github.com/LukasKalbertodt/penguin/compare/lib-v0.1.0...lib-v0.1.1
