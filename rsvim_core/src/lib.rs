//! The core library for the [RSVIM](https://github.com/rsvim/rsvim) editor.

pub mod buf;
pub mod cfg;
pub mod chan;
pub mod cli;
pub mod cmdltext;
pub mod coord;
pub mod err;
pub mod evloop;
pub mod handles;
pub mod hl;
pub mod js;
pub mod log;
pub mod prelude;
pub mod state;
pub mod syntax;
pub mod ui;
pub mod util;

// Only for unit test.
#[cfg(test)]
mod buf_tests;
#[cfg(test)]
mod cli_tests;
#[cfg(test)]
mod coord_tests;
#[cfg(test)]
mod evloop_tests;
#[cfg(test)]
mod hl_tests;
#[cfg(test)]
mod js_tests;
#[cfg(test)]
mod syntax_tests;
#[cfg(test)]
pub mod tests;
