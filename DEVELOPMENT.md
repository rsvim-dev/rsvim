# Development

## Toolchain

Please setup your development environment with:

- Latest C++ toolchain:
  - For Linux, install [gcc](https://gcc.gnu.org/).
  - For macOS, install [Xcode Clang](https://developer.apple.com/xcode/).
  - For Windows, install [Visual Studio with C++/C# Desktop Components](https://visualstudio.microsoft.com/), [LLVM](https://llvm.org/).
- Install [mise](https://github.com/jdx/mise), then run `mise i`. This installs
  Rust, Node, Python and all the command line tools used below, including
  [just](https://github.com/casey/just), `cargo-nextest`, `sccache`, `taplo`,
  `typos` and `cargo-release`.

## Repository

Sync git submodules with below commands:

- Run `git submodule update --init --recursive` to update all the recursive submodules.
- Run `git submodule update --remote --merge` to update to latest commit.

## Commands

All development commands are defined in the [justfile](./justfile). Run `just`
to list the recipes, and `just --usage RECIPE` to see a recipe's parameters.
Extra arguments are forwarded to the underlying tool, for example
`just build --release` or `just test buf`.

| Command            | Description                                                        |
| ------------------ | ------------------------------------------------------------------ |
| `just build`       | `cargo build`, extra arguments are forwarded.                      |
| `just build-msrv`  | `cargo +1.90 build`, builds with the minimum supported Rust.       |
| `just clippy`      | `cargo clippy --workspace --all-targets`.                          |
| `just clippy-msrv` | `cargo +1.90 clippy --workspace --all-targets`.                    |
| `just test`        | `cargo nextest run`, runs all tests when no filter is given.       |
| `just test-msrv`   | `cargo +1.90 nextest run`.                                         |
| `just test-list`   | `cargo nextest list`, lists tests without running them.            |
| `just miri`        | `cargo +nightly miri nextest run` for `rsvim_core`.                |
| `just bench NAME`  | `cargo bench -p rsvim_core --bench NAME`.                          |
| `just typos`       | Check spelling with `typos`.                                       |
| `just fmt`         | Format TOML, Markdown, TypeScript and Rust.                        |
| `just fmt-check`   | Same as `fmt`, but only check and exit non-zero on differences.    |
| `just bundle`      | Compile TypeScript to JavaScript and bundle the runtime.           |
| `just doc-check`   | `cargo doc --workspace`, builds the documentation.                 |
| `just doc`         | Builds rustdoc, then serves `target/doc` with browser-sync.        |
| `just release`     | `cargo release`, publishes `rsvim_macro` then the other crates.    |
| `just npmpack`     | Bump the npm package version in `package.json`.                    |
| `just ci`          | Run the CI checks locally: bundle, fmt-check, typos, clippy, test. |

A few tips:

- `sccache` is enabled by default. Disable it for a single run with
  `just RUSTC_WRAPPER="" clippy`.
- `RSVIM_LOG` defaults to `trace`; override it with
  `RSVIM_LOG=debug just test`.
- To debug the binary, run
  `RUST_BACKTRACE=full RSVIM_LOG=trace ./target/debug/rsvim`. It writes all the
  logs to a file named `rsvim_YYYY-MM-DD_HH-mm-ss-SSS.log`.
