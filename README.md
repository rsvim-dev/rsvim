<p align="center">
  <img alt="logo.svg" src="https://cdn.jsdelivr.net/gh/rsvim-dev/assets@main/logo/RSVIM-logo.svg" />
</p>

<p align="center">
The VIM editor reinvented in Rust+TypeScript.
</p>

<p align="center">
  <a href="https://crates.io/crates/rsvim"><img alt="crates" src="https://img.shields.io/crates/v/rsvim" /></a>
  <a href="https://www.npmjs.com/package/rsvim-types"><img alt="npmjs" src="https://img.shields.io/npm/v/rsvim-types" /></a>
  <a href="https://docs.rs/rsvim_core/latest/"><img alt="rustdocs" src="https://img.shields.io/docsrs/rsvim_core?label=docs.rs" /></a>
  <!-- a href="https://github.com/rsvim-dev/rsvim/actions/workflows/release.yml"><img alt="release.yml" src="https://img.shields.io/github/actions/workflow/status/rsvim-dev/rsvim/release.yml" /></a-->
  <a href="https://github.com/rsvim-dev/rsvim/actions/workflows/ci.yml"><img alt="ci.yml" src="https://img.shields.io/github/actions/workflow/status/rsvim-dev/rsvim/ci.yml?branch=main&label=ci" /></a>
  <a href="https://app.codecov.io/gh/rsvim-dev/rsvim"><img alt="codecov" src="https://img.shields.io/codecov/c/github/rsvim-dev/rsvim" /></a>
  <a href="https://discord.gg/5KtRUCAByB"><img alt="discord" src="https://img.shields.io/discord/1220171472329379870?logo=discord&style=social&label=discord" /></a>
</p>

> [!CAUTION]
>
> _**This project is still in very early stage of development and not ready for use. Please choose alternatives [Neovim](https://neovim.io/) and [Vim](https://www.vim.org/).**_

## About

RSVIM is an open source terminal based text editor, built from scratch with [Rust](https://www.rust-lang.org/), [Tokio](https://tokio.rs/) and [V8](https://v8.dev/) javascript engine. It strives to be highly extensible by following main features, concepts, philosophy of ([Neo](https://neovim.io/))[Vim](https://www.vim.org/), while also to be:

- A fast editor that fully utilizes all CPU cores and never freezes.
- A powerful TUI engine that provides widgets, event handlers, layouts, etc.
- A consistent JavaScript-based runtime with built-in support for TypeScript.
- An editing service that allows multiple users to access remotely and work together.
- A text processing tool that integrates with shell environment.

## Installation

Please download prebuilt executables in releases page, or build with cargo:

```bash
cargo install rsvim
```

To get latest updates, build with source:

```bash
cargo install --path ./rsvim_cli
```

## Get Started

Please check out [Documentation](https://rsvim.pages.dev/) for more details!

## Contribution

Contributions to RSVIM are always welcomed. A few guidelines that help quickly set up development environment can be found in "DEVELOPMENT.md".

Road map and high-level design can be found in [RFC](https://github.com/rsvim-dev/rfc), please submit your ideas and designs there if they need fairly large efforts.

## Credits

Some source code are studied from following projects for implementing the initial prototype of javascript runtime and [Minimum Common Web Platform API](https://min-common-api.proposal.wintertc.org/):

- **[dune](https://github.com/aalykiot/dune)**: A hobby runtime for JavaScript and TypeScript 🚀.
- **[deno](https://github.com/denoland/deno)**: A modern runtime for JavaScript and TypeScript.

## Supporting the Project

If you like RSVIM, please consider sponsoring it. Your support encourages contributors and maintainers of this project, and other fees or efforts spent on it.

- [GitHub Sponsor](https://github.com/sponsors/rsvim)
- [Open Collective](https://opencollective.com/rsvim)

## License

Licensed under "Vim License".
