# Changelog

All notable changes to the penguin **CLI app** will be documented here.


## [Unreleased]

## [0.2.2] - 2021-10-02

- Add feature `vendored-openssl` to compile `openssl` from source
  [PR #10](https://github.com/LukasKalbertodt/penguin/pull/10) (Thanks @philipahlberg)
- Updated the library to v0.1.5 ⇒ [check its changelog](../lib/CHANGELOG.md#014---2021-09-02).
- Updated other dependencies.

## [0.2.2] - 2021-10-02

- Updated the library to v0.1.4 ⇒ [check its changelog](../lib/CHANGELOG.md#014---2021-09-02).
- Updated other dependencies.


## [0.2.1] - 2021-07-18

Updated the library to v0.1.3 ⇒ [check its changelog](../lib/CHANGELOG.md#013---2021-07-18).


## [0.2.0] - 2021-05-11

Updated the library to v0.1.2 ⇒ [check its changelog](../lib/CHANGELOG.md#012---2021-05-10).

### Breaking
- Mounted file system paths are now automatically watched for file changes. The
  browser sessions will reload automatically if anything changes.

### Added
- Add `--open` flag to open the browser automatically.
- Add `--no-auto-watch` flag to disable auto watch behavior.
- Add `-w/--watch` option to specify additional watched paths.
- Add `--debounce` flag to set the debounce duration for watched paths.


## 0.1.0 - 2021-03-03
### Added
- Everything


[Unreleased]: https://github.com/LukasKalbertodt/penguin/compare/app-v0.2.3...HEAD
[0.2.3]: https://github.com/LukasKalbertodt/penguin/compare/app-v0.2.2...app-v0.2.3
[0.2.2]: https://github.com/LukasKalbertodt/penguin/compare/app-v0.2.1...app-v0.2.2
[0.2.1]: https://github.com/LukasKalbertodt/penguin/compare/app-v0.2.0...app-v0.2.1
[0.2.0]: https://github.com/LukasKalbertodt/penguin/compare/app-v0.1.0...app-v0.2.0
