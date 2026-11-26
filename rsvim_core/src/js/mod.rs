//! JavaScript runtime.
//!
//! The engine is QuickJS-ng, driven through [`rquickjs`]. Everything the rest of
//! the crate is allowed to touch is re-exported below, so `crate::js::JsKernel`
//! resolves without naming the engine.
//!
//! The engine is a *synchronous object*, [`JsHost`], and the event loop is the
//! only scheduler:
//!
//! - Nothing runs javascript except [`JsHost::enter`]. It runs the call, drains
//!   the microtask queue, reports unhandled rejections once, and marks the
//!   editor dirty -- so the rules that used to be repeated at every call site
//!   live in one function. See [`host`].
//! - An asynchronous binding does not give the engine a future. It creates a
//!   promise and runs the work as a plain tokio task; the result comes back as
//!   a message on the channel the editor already listens on, and the loop
//!   settles the promise. See [`ops`].
//! - Timers are a table the loop waits on, not futures. See [`timers`].
//! - QuickJS walks a module graph itself: `Module::eval` calls back into the
//!   [`loader`] for every `import`, so there is no `fetch_module_tree` /
//!   `ModuleMap` machinery to maintain.
//!
//! References:
//! - QuickJS-ng: <https://github.com/quickjs-ng/quickjs>.
//! - rquickjs: <https://github.com/DelSkayn/rquickjs>.
//! - Roll your own JavaScript runtime: <https://deno.com/blog/roll-your-own-javascript-runtime>.
//! - A hobby runtime for javascript/typescript: <https://github.com/aalykiot/dune>.

pub mod binding;
pub mod cmdline;
pub mod command;
pub mod converter;
pub mod core_modules;
pub mod err;
pub mod fs;
pub mod fs_loader;
pub mod host;
pub mod ids;
pub mod loader;
pub mod module;
pub mod ops;
pub mod process_command;
pub mod resource;
pub mod runtime;
pub mod timers;

pub use core_modules::CORE_MODULES;
pub use host::JsHost;
pub use ids::TaskId;
pub use ids::TimerId;
pub use runtime::JsKernel;
pub use runtime::JsKernelRc;
pub use runtime::JsRuntimeOptions;
pub use runtime::capture_exception;
pub use runtime::get_state;
pub use runtime::set_state;
pub use runtime::to_io_error;

/// Get the bundled QuickJS-ng engine version string (e.g. `2024-01-13`).
///
/// This is the successor of the old V8-era `v8_version()`. The version
/// lives in the vendored quickjs C sources and is exposed through
/// `JS_GetVersion`, which returns a pointer to a compile-time constant.
pub fn quickjs_version() -> &'static str {
  unsafe {
    let ptr = rquickjs::qjs::JS_GetVersion();
    if ptr.is_null() {
      "unknown"
    } else {
      std::ffi::CStr::from_ptr(ptr).to_str().unwrap_or("unknown")
    }
  }
}

#[cfg(test)]
mod buf_tests;
#[cfg(test)]
mod cmd_tests;
#[cfg(test)]
mod fs_loader_tests;
#[cfg(test)]
mod fs_tests;
#[cfg(test)]
mod host_tests;
#[cfg(test)]
mod microtask_tests;
#[cfg(test)]
mod module_map_tests;
#[cfg(test)]
mod opt_tests;
#[cfg(test)]
mod process_tests;
#[cfg(test)]
mod syn_tests;
#[cfg(test)]
mod text_encoder_tests;
#[cfg(test)]
mod timeout_tests;
