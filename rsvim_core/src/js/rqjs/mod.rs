//! QuickJS-ng backend, built on `rquickjs`.
//!
//! This module is additive: enabling `js-quickjs` does not turn `js-v8` off, so
//! the crate keeps building against the engine that ships while the port grows.
//! Once the port reaches parity the re-export in [`super`] flips and `js-v8`
//! stops being the default (see `quickjs-plan.md` Step 6/7).

pub mod runtime;

pub use runtime::*;
