# Changelog

All notable changes to the penguin **CLI app** will be documented here.


## [Unreleased]


## [0.2.6] - 2022-11-26
- Improve `cargo doc` workflow by treating `remove` file system events as less important in watcher. Adds `--removal-debounce` flag. See [385b63](https://github.com/LukasKalbertodt/penguin/commit/385b6395142aff28fa5063162a8023e1392b0cf1).
- Updated the library to v0.1.8 ⇒ [check its changelog](../lib/CHANGELOG.md#017---2022-06-22).
  - Add basic HTTP range request support for the file server. With this, video files served by Penguin can be played by Safari.
  - Add body sniffing to detect HTML content (and insert reload script) more often (see #11)

## [0.2.5] - 2022-06-22

- Updated the library to v0.1.7 ⇒ [check its changelog](../lib/CHANGELOG.md#017---2022-06-22).

## [0.2.4] - 2022-06-08

- Updated the library to v0.1.6 ⇒ [check its changelog](../lib/CHANGELOG.md#016---2022-06-08).
- Updated other dependencies.

## [0.2.3] - 2021-10-02

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


[Unreleased]: https://github.com/LukasKalbertodt/penguin/compare/app-v0.2.6...HEAD
[0.2.6]: https://github.com/LukasKalbertodt/penguin/compare/app-v0.2.5...app-v0.2.6
[0.2.5]: https://github.com/LukasKalbertodt/penguin/compare/app-v0.2.4...app-v0.2.5
[0.2.4]: https://github.com/LukasKalbertodt/penguin/compare/app-v0.2.3...app-v0.2.4
[0.2.3]: https://github.com/LukasKalbertodt/penguin/compare/app-v0.2.2...app-v0.2.3
[0.2.2]: https://github.com/LukasKalbertodt/penguin/compare/app-v0.2.1...app-v0.2.2
[0.2.1]: https://github.com/LukasKalbertodt/penguin/compare/app-v0.2.0...app-v0.2.1
[0.2.0]: https://github.com/LukasKalbertodt/penguin/compare/app-v0.1.0...app-v0.2.0
