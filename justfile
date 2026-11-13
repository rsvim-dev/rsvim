# Development toolkit.
#
#   just                     # list all recipes
#   just build -r            # extra arguments are passed through to cargo
#   just RUSTC_WRAPPER="" clippy   # disable sccache for one run
#
# Run `just --usage RECIPE` for a recipe's usage plus parameter details.

msrv := "1.90.0"

# `v8` prebuilt static libraries use the static CRT (`/MT`), so Rust has to link
# it too. `RUSTFLAGS` replaces the target rustflags in `.cargo/config.toml`,
# hence we replicate them here on Windows.
export RUSTFLAGS := "-Dwarnings" + if os() == "windows" { " -Ctarget-feature=+crt-static" } else { "" }
export RUSTDOCFLAGS := "-Dwarnings"
export RUST_BACKTRACE := "full"
export RSVIM_LOG := env("RSVIM_LOG", "trace")
export MIRIFLAGS := "-Zmiri-backtrace=full -Zmiri-disable-isolation -Zmiri-permissive-provenance"

# `sccache` is installed by `mise i`. Cargo treats an empty `RUSTC_WRAPPER` as
# unset, so it can be disabled with `just RUSTC_WRAPPER="" clippy`.
export RUSTC_WRAPPER := env("RUSTC_WRAPPER", "sccache")

alias b := build
alias c := clippy
alias t := test
alias f := fmt

[doc("""
List all recipes

Use `just --usage RECIPE` to also see parameter details.
""")]
default:
    @just --list

[private]
_env:
    @echo "RUSTFLAGS      = {{ RUSTFLAGS }}"
    @echo "RUSTDOCFLAGS   = {{ RUSTDOCFLAGS }}"
    @echo "RUST_BACKTRACE = {{ RUST_BACKTRACE }}"
    @echo "RSVIM_LOG      = {{ RSVIM_LOG }}"
    @echo "MIRIFLAGS      = {{ MIRIFLAGS }}"
    @echo "RUSTC_WRAPPER  = {{ RUSTC_WRAPPER }}"

[arg("args", help="extra arguments passed to `cargo build`, e.g. --release, --profile nightly, -F foo, --no-default-features")]
[doc("""
cargo build

Build the workspace with `cargo build`. Extra arguments are forwarded to cargo:
  --release / -r          release profile
  --profile nightly       nightly profile
  -F, --features FEATURES enable features
  --no-default-features   disable default features
  -v, --verbose           verbose output
Use `build-msrv` for the minimal rustc version.
""")]
build *args: _env
    cargo build {{ args }}

[arg("args", help="extra arguments passed to `cargo +1.90 build`")]
[doc("""
cargo build with the minimal rustc version

Same as `build` but compiles with the minimum supported Rust version 1.90.0,
i.e. `cargo +1.90 build`, to verify the MSRV declared in Cargo.toml.
""")]
build-msrv *args: _env
    cargo +{{ msrv }} build {{ args }}

[arg("args", help="extra arguments passed to `cargo clippy`")]
[doc("""
cargo clippy

Lint the workspace with all targets: `cargo clippy --workspace --all-targets`.
Extra arguments such as `--verbose` or `-p rsvim_core` are forwarded to cargo.
Use `clippy-msrv` for the minimal rustc version.
""")]
clippy *args: _env
    cargo clippy --workspace --all-targets {{ args }}

[arg("args", help="extra arguments passed to `cargo +1.90 clippy`")]
[doc("""
cargo clippy with the minimal rustc version

Same as `clippy` but lints with the minimum supported Rust version 1.90.0, i.e.
`cargo +1.90 clippy --workspace --all-targets`.
""")]
clippy-msrv *args: _env
    cargo +{{ msrv }} clippy --workspace --all-targets {{ args }}

[arg("args", help="test names or filters, e.g. `just test buf`; runs all tests when empty")]
[doc("""
cargo nextest run

Run tests with `cargo nextest run --no-capture`. With no argument all tests run
(`--all`), otherwise the arguments are passed as names/filters.
Sets RUST_BACKTRACE=full and RSVIM_LOG (defaults to trace, override with
`RSVIM_LOG=debug just test`).
Use `test-list` to list tests and `test-msrv` for the minimal rustc version.
""")]
test *args: _env
    cargo nextest run --no-capture {{ if args == "" { "--all" } else { args } }}

[arg("args", help="test names or filters; runs all tests when empty")]
[doc("""
cargo nextest run with the minimal rustc version

Same as `test` but runs with the minimum supported Rust version 1.90.0, i.e.
`cargo +1.90 nextest run`.
""")]
test-msrv *args: _env
    cargo +{{ msrv }} nextest run --no-capture {{ if args == "" { "--all" } else { args } }}

[arg("args", help="extra arguments passed to `cargo nextest list`, e.g. --workspace")]
[doc("""
List tests

List every test with `cargo nextest list --no-pager` without running them.
Extra arguments are forwarded to cargo nextest.
""")]
test-list *args: _env
    cargo nextest list --no-pager {{ args }}

[arg("args", help="extra arguments passed to `cargo miri nextest`, e.g. -j 4")]
[doc("""
cargo +nightly miri test

Run the `rsvim_core` tests under Miri:
`cargo +nightly miri nextest run --no-default-features -F unicode -p rsvim_core`.
Extra arguments are forwarded, e.g. `-j N` to limit the job count.
Sets MIRIFLAGS (-Zmiri-backtrace=full, -Zmiri-disable-isolation,
-Zmiri-permissive-provenance) and RUST_BACKTRACE=full.
""")]
miri *args: _env
    cargo +nightly miri nextest run {{ args }} --no-default-features -F unicode -p rsvim_core

[arg("name", help="benchmark name, matching a `[[bench]]` target in rsvim_core")]
[doc("""
cargo bench

Run one criterion benchmark from `rsvim_core`:
`cargo bench -p rsvim_core --bench NAME`, where NAME must match a [[bench]]
target in rsvim_core/Cargo.toml.
""")]
bench name: _env
    cargo bench -p rsvim_core --bench {{ name }}

[doc("""
Check spelling with typos

Run `typos` to check spelling across the repository using typos.toml, and exit
non-zero when a typo is found. Also called by `ci`.
""")]
typos: _env
    typos

[doc("""
Code formatters

Format all code: taplo (TOML), prettier (Markdown and TypeScript), rustfmt
(`cargo +nightly fmt`). Spelling is checked separately by `just typos`.
""")]
fmt: _env
    taplo fmt
    npx prettier --write *.md ./src/*.ts
    cargo +nightly fmt

[doc("""
Check formatting without modifying files

Same formatters as `fmt` but with `--check`, exiting non-zero when a file is
not formatted. Used by CI and by `just ci`.
""")]
fmt-check: _env
    taplo check
    taplo fmt --check
    npx prettier --check *.md ./src/*.ts
    cargo +nightly fmt --check

[doc("""
Compile typescript and bundle the javascript runtime

`tsc` compiles the .ts files into ./build/ and ./types/, `rollup` tree-shakes
and bundles them (see rollup.config.js), and `terser` minifies the bundles into
rsvim_core/js/*.min.js (see terser.config.json).
""")]
bundle: _env
    npx tsc
    npx rollup -c
    npx terser ./build/00__web.bundle.js --config-file terser.config.json -o ./rsvim_core/js/00__web.min.js
    npx terser ./build/01__rsvim.bundle.js --config-file terser.config.json -o ./rsvim_core/js/01__rsvim.min.js

[doc("""
cargo doc --workspace

Build the workspace rustdoc without serving it, e.g. to check documentation
warnings. Sets RUSTDOCFLAGS=-Dwarnings.
""")]
doc-check: _env
    cargo doc --workspace

[doc("""
cargo doc and serve

Build the rustdoc documentation, then serve target/doc with browser-sync (the
server keeps running until interrupted). Sets RUSTDOCFLAGS=-Dwarnings.
""")]
doc: _env
    cargo doc && npx browser-sync start --ss target/doc -s target/doc --directory --startPath rsvim_core --no-open

[arg("args", help="extra arguments passed to `cargo release`, e.g. --execute --no-verify")]
[arg("level", pattern="alpha|beta|rc|major|minor|patch", help="release level: alpha, beta, rc, major, minor or patch")]
[doc("""
cargo release

Publish the crates to crates.io from the git repository root, publishing
rsvim_macro first (it is used by the other crates) and then rsvim_core and
rsvim. By default this is a dry run; pass `--execute` (and usually
`--no-verify`) to actually publish, e.g.
`just release patch --execute --no-verify`.
Fails when the working tree is dirty or the branch is not allowed by
release.toml.
""")]
release level *args: _env
    cargo release --package rsvim_macro {{ level }} {{ args }}
    cargo release --package rsvim_core --package rsvim {{ level }} {{ args }}

[arg("level", pattern="major|minor|patch", help="semver bump level: major, minor or patch")]
[doc("""
npm bump the package version

Bump the version in package.json with
`npm version LEVEL --no-git-tag-version`, e.g. `just npmpack patch`. This does
not commit or tag (--no-git-tag-version) and does not publish.
""")]
npmpack level: _env
    npm version {{ level }} --no-git-tag-version

[doc("""
Run CI checks locally

Run what the GitHub Actions CI runs, in order: bundle the JavaScript runtime
and verify no file changed, check formatting (fmt-check), spelling (typos),
clippy and the test suite.
""")]
ci: bundle
    git diff --exit-code HEAD
    just fmt-check
    just typos
    just clippy
    just test
