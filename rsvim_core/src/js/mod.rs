//! JavaScript runtime.
//!
//! The engine is a build-time choice: [`v8`] is the mature one and is what
//! ships, [`rqjs`] is the QuickJS-ng port. Nothing outside this module is
//! allowed to name an engine type: everything the rest of the crate sees is
//! re-exported below, so `crate::js::JsKernel` resolves to whichever engine the
//! build selected.
//!
//! The port is **additive** while it is being built, on purpose. `rqjs` is a
//! namespaced module (`crate::js::rqjs::*`) rather than a glob re-export,
//! because the rest of the crate -- `chan`, `evloop`, the bindings -- is still
//! shaped around v8 and cannot be switched in one commit. Building both engines
//! at once is what lets the port grow step by step: each step lands with the
//! crate still compiling, and the flip to `pub use rqjs::*` is the last step
//! rather than the first (see `quickjs-plan.md` §6).
//!
//! References:
//! - Embed V8: <https://v8.dev/docs/embed>.
//! - Roll your own JavaScript runtime: <https://deno.com/blog/roll-your-own-javascript-runtime>.
//! - Learning V8 Tutorial: <https://github.com/danbev/learning-v8>.
//! - A hobby runtime for javascript/typescript: <https://github.com/aalykiot/dune>.
//! - V8 API Reference: <https://v8docs.nodesource.com/node-24.1/index.html>.

#[cfg(not(any(feature = "js-v8", feature = "js-quickjs")))]
compile_error!(
  "one javascript engine must be selected: enable either `js-v8` or \
   `js-quickjs`"
);

pub mod cmdline;
pub mod command;
pub mod core_modules;
pub mod fs;
pub mod ids;
pub mod process_command;
pub mod syn;

pub use core_modules::CORE_MODULES;
pub use ids::TaskId;
pub use ids::TimerId;

#[cfg(feature = "js-v8")]
pub mod v8;
#[cfg(feature = "js-v8")]
pub use v8::*;

/// The QuickJS-ng backend.
///
/// Deliberately a namespace, not a re-export: see the module doc above and
/// [`super`].
#[cfg(feature = "js-quickjs")]
pub mod rqjs;
