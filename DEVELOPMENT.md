# Development

## Toolchain

Please setup your development environment with:

- Latest C++ toolchain:
  - For Linux, install [gcc](https://gcc.gnu.org/).
  - For macOS, install [Xcode Clang](https://developer.apple.com/xcode/).
  - For Windows, install [Visual Studio with C++/C# Desktop Components](https://visualstudio.microsoft.com/).
- Install [rust](https://rust-lang.org/tools/install/) and [cargo-binstall](https://github.com/cargo-bins/cargo-binstall), then run `cargo binstall -y cargo-nextest cargo-release git-cliff sccache taplo-cli typos-cli` to install below tools:
  - [cargo-nextest](https://github.com/nextest-rs/nextest)
  - [cargo-release](https://github.com/crate-ci/cargo-release)
  - [git-cliff](https://github.com/orhun/git-cliff)
  - [sccache](https://github.com/mozilla/sccache)
  - [taplo-cli](https://github.com/tamasfe/taplo)
  - [typos-cli](https://github.com/crate-ci/typos)
- Install [mise](https://github.com/jdx/mise), then run `mise i`.
- (Optional) Faster linker for linux, install [clang](https://llvm.org/), [mold](https://github.com/rui314/mold).

## Repository

Sync git submodules with below commands:

- Run `git submodule update --init --recursive` to update all the recursive submodules.
- Run `git submodule update --remote --merge` to update to latest commit.

## Rust

The `dev.py` script is provided to help running cargo commands, use `dev.py -h` for more details. For window, please use `dev.cmd`.

- To lint code, please use `./dev.py clippy` (`cargo clippy`).
- To format code, please use `./dev.py fmt` (`cargo +nightly fmt`, `tsc`, `prettier`, etc).
- To run unit test, please use `./dev.py test` (`cargo test`).
- To debug code, please run binary with `RUST_BACKTRACE=full RSVIM_LOG=trace ./target/debug/rsvim`, it enables all the logs to a logging file named with format `rsvim_YYYY-MM-DD_HH-mm-ss-SSS.log`.
- To write docs, please use `./dev.py doc` (`cargo doc`).
- To release a new version, please use `./dev.py release [LEVEL]` (`cargo release`).

## TypeScript/JavaScript

- To transpile/compile typescript code to javascript code, please run `tsc`.
