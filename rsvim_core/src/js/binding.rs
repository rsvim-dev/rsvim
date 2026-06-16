//! Js runtime bindings.

pub mod global_rsvim;
pub mod global_this;

use crate::prelude::*;
// use crate::dns;
// use crate::exceptions;
// use crate::file;
// use crate::http_parser;
// use crate::js::report_and_exit;
// use crate::net;
// use crate::perf_hooks;
// use crate::process;
// use crate::promise;
// use crate::js::{check_exceptions, JsRuntime};
// use crate::signals;
// use crate::stdio;
// use crate::timers;
// use crate::prelude::*;
use std::ffi::c_void;

// /// Function pointer for the bindings initializers.
// type BindingInitFn = fn(&mut v8::PinScope<'s, 'b>) -> v8::Global<v8::Object>;
//
// lazy_static! {
//   pub static ref BINDINGS: FoldMap<&'static str, BindingInitFn> = {
//     let bindings: Vec<(&'static str, BindingInitFn)> = vec![
//       ("stdio", stdio::initialize),
//       ("timers", timers::initialize),
//       ("fs", file::initialize),
//       ("perf_hooks", perf_hooks::initialize),
//       ("dns", dns::initialize),
//       ("net", net::initialize),
//       ("promise", promise::initialize),
//       ("http_parser", http_parser::initialize),
//       ("signals", signals::initialize),
//       ("exceptions", exceptions::initialize),
//     ];
//     FoldMap::from_iter(bindings.into_iter())
//   };
// }

/// Populates a new JavaScript context with low-level Rust bindings.
pub fn create_new_context<'s, 'b>(
  scope: &mut v8::PinScope<'s, 'b, ()>,
) -> v8::Local<'s, v8::Context> {
  // Create and enter a new JavaScript context.
  let context = v8::Context::new(scope, Default::default());
  let global = context.global(scope);
  let scope = &mut v8::ContextScope::new(scope, context);

  // set_function_to(scope, global, "print", global_print);
  // set_function_to(scope, global, "$$reportError", global_report_error);
  // set_function_to(scope, global, "$$queueMicrotask", global_queue_micro);

  // Register the `__InternalRsvimGlobalObject` global object.
  let vim = create_object_under(scope, global, "__InternalRsvimGlobalObject");

  // For `globalThis`
  {
    set_function_to(
      scope,
      vim,
      "global_create_timer",
      global_this::timeout::create_timer,
    );
    set_function_to(
      scope,
      vim,
      "global_clear_timer",
      global_this::timeout::clear_timer,
    );
    set_function_to(
      scope,
      vim,
      "global_encoding_encode",
      global_this::text_encoder::encode,
    );
    set_function_to(
      scope,
      vim,
      "global_encoding_encode_into",
      global_this::text_encoder::encode_into,
    );
    set_function_to(
      scope,
      vim,
      "global_encoding_check_encoding_label",
      global_this::text_encoder::check_encoding_label,
    );
    set_function_to(
      scope,
      vim,
      "global_encoding_decode_single",
      global_this::text_encoder::decode_single,
    );
    set_function_to(
      scope,
      vim,
      "global_encoding_create_stream_decoder",
      global_this::text_encoder::create_stream_decoder,
    );
    set_function_to(
      scope,
      vim,
      "global_encoding_decode_stream",
      global_this::text_encoder::decode_stream,
    );
    set_function_to(
      scope,
      vim,
      "global_encoding_close_stream_decoder",
      global_this::text_encoder::close_stream_decoder,
    );
    set_function_to(
      scope,
      vim,
      "global_queue_microtask",
      global_this::microtask::queue_microtask,
    );
    set_function_to(
      scope,
      vim,
      "global_report_error",
      global_this::microtask::report_error,
    );
  }

  // For `Rsvim.buf`
  {
    set_function_to(
      scope,
      vim,
      "buf_write_sync",
      global_rsvim::buf::write_sync,
    );
    set_function_to(scope, vim, "buf_current", global_rsvim::buf::current);
    set_function_to(scope, vim, "buf_list", global_rsvim::buf::list);
  }

  // For `Rsvim.cmd`
  {
    set_function_to(scope, vim, "cmd_create", global_rsvim::cmd::create);
    set_function_to(scope, vim, "cmd_echo", global_rsvim::cmd::echo);
    set_function_to(scope, vim, "cmd_list", global_rsvim::cmd::list);
    set_function_to(scope, vim, "cmd_get", global_rsvim::cmd::get);
    set_function_to(scope, vim, "cmd_remove", global_rsvim::cmd::remove);
  }

  // For `Rsvim.fs`
  {
    set_function_to(scope, vim, "fs_open", global_rsvim::fs::open);
    set_function_to(scope, vim, "fs_open_sync", global_rsvim::fs::open_sync);
    set_function_to(scope, vim, "fs_close", global_rsvim::fs::close);
    set_function_to(scope, vim, "fs_read", global_rsvim::fs::read);
    set_function_to(scope, vim, "fs_read_sync", global_rsvim::fs::read_sync);
    set_function_to(scope, vim, "fs_write", global_rsvim::fs::write);
    set_function_to(scope, vim, "fs_write_sync", global_rsvim::fs::write_sync);
    set_function_to(scope, vim, "fs_read_file", global_rsvim::fs::read_file);
    set_function_to(
      scope,
      vim,
      "fs_read_file_sync",
      global_rsvim::fs::read_file_sync,
    );
    set_function_to(
      scope,
      vim,
      "fs_read_text_file",
      global_rsvim::fs::read_text_file,
    );
    set_function_to(
      scope,
      vim,
      "fs_read_text_file_sync",
      global_rsvim::fs::read_text_file_sync,
    );
    set_function_to(scope, vim, "fs_lstat", global_rsvim::fs::lstat);
    set_function_to(scope, vim, "fs_lstat_sync", global_rsvim::fs::lstat_sync);
    set_function_to(scope, vim, "fs_stat", global_rsvim::fs::stat);
    set_function_to(scope, vim, "fs_stat_sync", global_rsvim::fs::stat_sync);
    set_function_to(scope, vim, "fs_symlink", global_rsvim::fs::symlink);
    set_function_to(
      scope,
      vim,
      "fs_symlink_sync",
      global_rsvim::fs::symlink_sync,
    );
    set_function_to(scope, vim, "fs_link", global_rsvim::fs::link);
    set_function_to(scope, vim, "fs_link_sync", global_rsvim::fs::link_sync);
    set_function_to(scope, vim, "fs_mkdir", global_rsvim::fs::mkdir);
    set_function_to(scope, vim, "fs_mkdir_sync", global_rsvim::fs::mkdir_sync);
  }

  // For `Rsvim.opt`
  {
    set_function_to(scope, vim, "opt_get_wrap", global_rsvim::opt::get_wrap);
    set_function_to(scope, vim, "opt_set_wrap", global_rsvim::opt::set_wrap);
    set_function_to(
      scope,
      vim,
      "opt_get_line_break",
      global_rsvim::opt::get_line_break,
    );
    set_function_to(
      scope,
      vim,
      "opt_set_line_break",
      global_rsvim::opt::set_line_break,
    );
    set_function_to(
      scope,
      vim,
      "opt_get_tab_stop",
      global_rsvim::opt::get_tab_stop,
    );
    set_function_to(
      scope,
      vim,
      "opt_set_tab_stop",
      global_rsvim::opt::set_tab_stop,
    );
    set_function_to(
      scope,
      vim,
      "opt_get_expand_tab",
      global_rsvim::opt::get_expand_tab,
    );
    set_function_to(
      scope,
      vim,
      "opt_set_expand_tab",
      global_rsvim::opt::set_expand_tab,
    );
    set_function_to(
      scope,
      vim,
      "opt_get_shift_width",
      global_rsvim::opt::get_shift_width,
    );
    set_function_to(
      scope,
      vim,
      "opt_set_shift_width",
      global_rsvim::opt::set_shift_width,
    );
    set_function_to(
      scope,
      vim,
      "opt_get_file_encoding",
      global_rsvim::opt::get_file_encoding,
    );
    set_function_to(
      scope,
      vim,
      "opt_set_file_encoding",
      global_rsvim::opt::set_file_encoding,
    );
    set_function_to(
      scope,
      vim,
      "opt_get_file_format",
      global_rsvim::opt::get_file_format,
    );
    set_function_to(
      scope,
      vim,
      "opt_set_file_format",
      global_rsvim::opt::set_file_format,
    );
    set_function_to(
      scope,
      vim,
      "opt_get_fix_end_of_line",
      global_rsvim::opt::get_fix_end_of_line,
    );
    set_function_to(
      scope,
      vim,
      "opt_set_fix_end_of_line",
      global_rsvim::opt::set_fix_end_of_line,
    );
    set_function_to(
      scope,
      vim,
      "opt_get_syntax_parser_lib_path",
      global_rsvim::opt::get_syntax_parser_lib_path,
    );
    set_function_to(
      scope,
      vim,
      "opt_set_syntax_parser_lib_path",
      global_rsvim::opt::set_syntax_parser_lib_path,
    );
  }

  // For `Rsvim.proc`
  {
    set_function_to(
      scope,
      vim,
      "proc_spawn_child",
      global_rsvim::proc::spawn_child,
    );
    set_function_to(
      scope,
      vim,
      "proc_read_text_from_child",
      global_rsvim::proc::read_text_from_child,
    );
    set_function_to(
      scope,
      vim,
      "proc_wait_child",
      global_rsvim::proc::wait_child,
    );
  }

  // For `Rsvim.rt`
  {
    set_function_to(scope, vim, "rt_exit", global_rsvim::rt::exit);
  }

  // For `Rsvim.syn`
  {
    set_function_to(
      scope,
      vim,
      "syn_load_parser",
      global_rsvim::syn::load_treesitter_parser,
    );
    set_function_to(
      scope,
      vim,
      "syn_load_parser_sync",
      global_rsvim::syn::load_parser_sync,
    );
    set_function_to(
      scope,
      vim,
      "syn_list_parsers",
      global_rsvim::syn::list_parsers,
    );
    set_function_to(
      scope,
      vim,
      "syn_get_parser_metadata",
      global_rsvim::syn::get_parser_metadata,
    );
  }

  context
}

/// Adds a property with the given name and value, into the given object.
pub fn set_property_to(
  scope: &mut v8::PinScope,
  target: v8::Local<v8::Object>,
  name: &str,
  value: v8::Local<v8::Value>,
) {
  let key = v8::String::new(scope, name).unwrap();
  target.set(scope, key.into(), value);
}

/// Adds a read-only property with the given name and value, into the given object.
pub fn set_constant_to(
  scope: &mut v8::PinScope,
  target: v8::Local<v8::Object>,
  name: &str,
  value: v8::Local<v8::Value>,
) {
  let key = v8::String::new(scope, name).unwrap();
  target.define_own_property(
    scope,
    key.into(),
    value,
    v8::PropertyAttribute::READ_ONLY,
  );
}

/// Adds a `Function` object which calls the given Rust function
pub fn set_function_to(
  scope: &mut v8::PinScope,
  target: v8::Local<v8::Object>,
  name: &str,
  callback: impl v8::MapFnTo<v8::FunctionCallback>,
) {
  let key = v8::String::new(scope, name).unwrap();
  let template = v8::FunctionTemplate::new(scope, callback);
  let val = template.get_function(scope).unwrap();

  target.set(scope, key.into(), val.into());
}

/// Creates an object with a given name under a `target` object.
pub fn create_object_under<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  target: v8::Local<v8::Object>,
  name: &str,
) -> v8::Local<'s, v8::Object> {
  let template = v8::ObjectTemplate::new(scope);
  let key = v8::String::new(scope, name).unwrap();
  let value = template.new_instance(scope).unwrap();

  target.set(scope, key.into(), value.into());
  value
}

/// Stores a Rust type inside a v8 object.
pub fn set_internal_ref<T>(
  scope: &mut v8::PinScope,
  target: v8::Local<v8::Object>,
  index: usize,
  data: T,
) -> *mut T {
  let boxed_ref = Box::new(data);
  let addr = Box::into_raw(boxed_ref);
  let v8_ext = v8::External::new(scope, addr as *mut c_void);

  target.set_internal_field(index, v8_ext.into());
  addr
}

/// Gets a previously stored Rust type from a v8 object.
pub fn get_internal_ref<'s, T>(
  scope: &mut v8::PinScope<'s, '_>,
  source: v8::Local<v8::Object>,
  index: usize,
) -> &'s mut T {
  let v8_ref = source.get_internal_field(scope, index).unwrap();
  let external = v8_ref.cast::<v8::External>();
  let value = external.value() as *mut T;

  unsafe { &mut *value }
}

/// Sets error code to exception if possible.
pub fn set_exception_code(
  scope: &mut v8::PinScope,
  exception: v8::Local<v8::Value>,
  error: &TheErr,
) {
  let exception = exception.to_object(scope).unwrap();
  match error {
    TheErr::LoadModuleFailed(_, e)
    | TheErr::SaveBufferFailed(_, _, e)
    | TheErr::OpenFileFailed(_, e) => {
      let key = v8::String::new(scope, "code").unwrap();
      let value = v8::String::new(scope, &format!("{:?}", e.kind())).unwrap();
      exception.set(scope, key.into(), value.into());
    }
    _ => {}
  }
}

/// Useful utility to throw v8 exceptions.
pub fn throw_exception(scope: &mut v8::PinScope, error: &TheErr) {
  let message = v8::String::new(scope, &error.to_string()).unwrap();
  let exception = v8::Exception::error(scope, message);
  set_exception_code(scope, exception, error);
  scope.throw_exception(exception);
}

/// Useful utility to throw v8 type errors.
pub fn throw_type_error(scope: &mut v8::PinScope, error: &TheErr) {
  let message = v8::String::new(scope, &error.to_string()).unwrap();
  let exception = v8::Exception::type_error(scope, message);
  scope.throw_exception(exception);
}

/// Useful utility to throw v8 range errors.
pub fn throw_range_error(scope: &mut v8::PinScope, error: &TheErr) {
  let message = v8::String::new(scope, &error.to_string()).unwrap();
  let exception = v8::Exception::range_error(scope, message);
  scope.throw_exception(exception);
}
