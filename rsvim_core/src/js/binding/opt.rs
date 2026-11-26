//! APIs for the `opt` module.
//!
//! Every function here is a plain getter or setter over `Editor` state, so the
//! rquickjs shape is much smaller than the v8 one: no `ReturnValue`, no
//! `FunctionCallbackArguments`, no `debug_assert!` on the argument count
//! (`rquickjs` checks parameters against the signature before calling). A getter
//! returns its value, a setter returns `()`.
//!
//! Arguments that v8 coerced with `to_integer` / `to_rust_string_lossy` are taken
//! as `Coerced<T>`, which performs the same `JS_ToNumber` / `JS_ToString` step.

use crate::buf::opt::FileEncodingOption;
use crate::buf::opt::FileFormatOption;
use crate::js::binding::set_function_to;
use crate::js::get_state;
use crate::prelude::*;
use compact_str::ToCompactString;
use rquickjs::Coerced;
use rquickjs::Ctx;
use rquickjs::Exception;
use rquickjs::Object;
use rquickjs::Result as JsResult;

/// Fetches the kernel, or raises a javascript error if the bindings were
/// installed on a context that never got one.
macro_rules! kernel {
  ($ctx:expr) => {
    get_state($ctx).ok_or_else(|| {
      Exception::throw_message($ctx, "js kernel is not initialised")
    })?
  };
}

/// `opt.getWrap()`.
pub fn get_wrap<'js>(ctx: Ctx<'js>) -> JsResult<bool> {
  let kernel = kernel!(&ctx);
  let tree = kernel.borrow().editor.tree.clone();
  let tree = lock!(tree);
  let value = tree.global_local_options().wrap();
  trace!("get_wrap: {:?}", value);
  Ok(value)
}

/// `opt.setWrap(value)`.
pub fn set_wrap<'js>(ctx: Ctx<'js>, value: bool) -> JsResult<()> {
  trace!("set_wrap: {:?}", value);
  let kernel = kernel!(&ctx);
  let tree = kernel.borrow().editor.tree.clone();
  let mut tree = lock!(tree);
  tree.global_local_options_mut().set_wrap(value);
  Ok(())
}

/// `opt.getLineBreak()`.
pub fn get_line_break<'js>(ctx: Ctx<'js>) -> JsResult<bool> {
  let kernel = kernel!(&ctx);
  let tree = kernel.borrow().editor.tree.clone();
  let tree = lock!(tree);
  let value = tree.global_local_options().line_break();
  trace!("get_line_break: {:?}", value);
  Ok(value)
}

/// `opt.setLineBreak(value)`.
pub fn set_line_break<'js>(ctx: Ctx<'js>, value: bool) -> JsResult<()> {
  trace!("set_line_break: {:?}", value);
  let kernel = kernel!(&ctx);
  let tree = kernel.borrow().editor.tree.clone();
  let mut tree = lock!(tree);
  tree.global_local_options_mut().set_line_break(value);
  Ok(())
}

/// `opt.getTabStop()`.
pub fn get_tab_stop<'js>(ctx: Ctx<'js>) -> JsResult<u32> {
  let kernel = kernel!(&ctx);
  let buffer_manager = kernel.borrow().editor.buffer_manager.clone();
  let buffer_manager = lock!(buffer_manager);
  let value = buffer_manager.global_local_options().tab_stop();
  trace!("get_tab_stop: {:?}", value);
  Ok(value as u32)
}

/// `opt.setTabStop(value)`.
pub fn set_tab_stop<'js>(ctx: Ctx<'js>, value: Coerced<i32>) -> JsResult<()> {
  let value = value.0;
  trace!("set_tab_stop: {:?}", value);
  if !(0..=u8::MAX as i32).contains(&value) {
    return Err(Exception::throw_range(
      &ctx,
      &format!("setTabStop: out of range: {}", value),
    ));
  }
  let kernel = kernel!(&ctx);
  let buffer_manager = kernel.borrow().editor.buffer_manager.clone();
  let mut buffer_manager = lock!(buffer_manager);
  buffer_manager
    .global_local_options_mut()
    .set_tab_stop(value as u8);
  Ok(())
}

/// `opt.getExpandTab()`.
pub fn get_expand_tab<'js>(ctx: Ctx<'js>) -> JsResult<bool> {
  let kernel = kernel!(&ctx);
  let buffer_manager = kernel.borrow().editor.buffer_manager.clone();
  let buffer_manager = lock!(buffer_manager);
  let value = buffer_manager.global_local_options().expand_tab();
  trace!("get_expand_tab: {:?}", value);
  Ok(value)
}

/// `opt.setExpandTab(value)`.
pub fn set_expand_tab<'js>(ctx: Ctx<'js>, value: bool) -> JsResult<()> {
  trace!("set_expand_tab: {:?}", value);
  let kernel = kernel!(&ctx);
  let buffer_manager = kernel.borrow().editor.buffer_manager.clone();
  let mut buffer_manager = lock!(buffer_manager);
  buffer_manager
    .global_local_options_mut()
    .set_expand_tab(value);
  Ok(())
}

/// `opt.getShiftWidth()`.
pub fn get_shift_width<'js>(ctx: Ctx<'js>) -> JsResult<u32> {
  let kernel = kernel!(&ctx);
  let buffer_manager = kernel.borrow().editor.buffer_manager.clone();
  let buffer_manager = lock!(buffer_manager);
  let value = buffer_manager.global_local_options().shift_width();
  trace!("get_shift_width: {:?}", value);
  Ok(value as u32)
}

/// `opt.setShiftWidth(value)`.
pub fn set_shift_width<'js>(
  ctx: Ctx<'js>,
  value: Coerced<i32>,
) -> JsResult<()> {
  let value = value.0;
  trace!("set_shift_width: {:?}", value);
  if !(0..=u8::MAX as i32).contains(&value) {
    return Err(Exception::throw_range(
      &ctx,
      &format!("setShiftWidth: out of range: {}", value),
    ));
  }
  let kernel = kernel!(&ctx);
  let buffer_manager = kernel.borrow().editor.buffer_manager.clone();
  let mut buffer_manager = lock!(buffer_manager);
  buffer_manager
    .global_local_options_mut()
    .set_shift_width(value as u8);
  Ok(())
}

/// `opt.getFileEncoding()`.
pub fn get_file_encoding<'js>(ctx: Ctx<'js>) -> JsResult<String> {
  let kernel = kernel!(&ctx);
  let buffer_manager = kernel.borrow().editor.buffer_manager.clone();
  let buffer_manager = lock!(buffer_manager);
  let value = buffer_manager.global_local_options().file_encoding();
  trace!("get_file_encoding: {:?}", value);
  Ok(value.to_compact_string().to_string())
}

/// `opt.setFileEncoding(value)`.
pub fn set_file_encoding<'js>(
  ctx: Ctx<'js>,
  value: Coerced<String>,
) -> JsResult<()> {
  // v8 lowercased before matching, so the js side may pass any casing.
  let value = value.0.to_lowercase();
  trace!("set_file_encoding: {:?}", value);
  let value = FileEncodingOption::try_from(value.as_str()).map_err(|_| {
    Exception::throw_range(
      &ctx,
      &format!("setFileEncoding: unknown encoding {:?}", value),
    )
  })?;
  let kernel = kernel!(&ctx);
  let buffer_manager = kernel.borrow().editor.buffer_manager.clone();
  let mut buffer_manager = lock!(buffer_manager);
  buffer_manager
    .global_local_options_mut()
    .set_file_encoding(value);
  Ok(())
}

/// `opt.getFileFormat()`.
pub fn get_file_format<'js>(ctx: Ctx<'js>) -> JsResult<String> {
  let kernel = kernel!(&ctx);
  let buffer_manager = kernel.borrow().editor.buffer_manager.clone();
  let buffer_manager = lock!(buffer_manager);
  let value = buffer_manager.global_local_options().file_format();
  trace!("get_file_format: {:?}", value);
  Ok(value.to_string())
}

/// `opt.setFileFormat(value)`.
pub fn set_file_format<'js>(
  ctx: Ctx<'js>,
  value: Coerced<String>,
) -> JsResult<()> {
  let value = value.0.to_lowercase();
  trace!("set_file_format: {:?}", value);
  let value = FileFormatOption::try_from(value.as_str()).map_err(|_| {
    Exception::throw_range(
      &ctx,
      &format!("setFileFormat: unknown format {:?}", value),
    )
  })?;
  let kernel = kernel!(&ctx);
  let buffer_manager = kernel.borrow().editor.buffer_manager.clone();
  let mut buffer_manager = lock!(buffer_manager);
  buffer_manager
    .global_local_options_mut()
    .set_file_format(value);
  Ok(())
}

/// `opt.getFixEndOfLine()`.
pub fn get_fix_end_of_line<'js>(ctx: Ctx<'js>) -> JsResult<bool> {
  let kernel = kernel!(&ctx);
  let buffer_manager = kernel.borrow().editor.buffer_manager.clone();
  let buffer_manager = lock!(buffer_manager);
  let value = buffer_manager.global_local_options().fix_end_of_line();
  trace!("get_fix_end_of_line: {:?}", value);
  Ok(value)
}

/// `opt.setFixEndOfLine(value)`.
pub fn set_fix_end_of_line<'js>(ctx: Ctx<'js>, value: bool) -> JsResult<()> {
  trace!("set_fix_end_of_line: {:?}", value);
  let kernel = kernel!(&ctx);
  let buffer_manager = kernel.borrow().editor.buffer_manager.clone();
  let mut buffer_manager = lock!(buffer_manager);
  buffer_manager
    .global_local_options_mut()
    .set_fix_end_of_line(value);
  Ok(())
}

/// `opt.getSyntaxParserLibPath()`.
pub fn get_syntax_parser_lib_path<'js>(ctx: Ctx<'js>) -> JsResult<String> {
  let kernel = kernel!(&ctx);
  let syntax_manager = kernel.borrow().editor.syntax_manager.clone();
  let syntax_manager = lock!(syntax_manager);
  let value = syntax_manager
    .loader()
    .treesitter_parser_lib_path()
    .to_string_lossy()
    .to_string();
  trace!("get_syntax_parser_lib_path: {:?}", value);
  Ok(value)
}

/// `opt.setSyntaxParserLibPath(value)`.
pub fn set_syntax_parser_lib_path<'js>(
  ctx: Ctx<'js>,
  value: Coerced<String>,
) -> JsResult<()> {
  let value = value.0;
  trace!("set_syntax_parser_lib_path: {:?}", value);
  let kernel = kernel!(&ctx);
  let syntax_manager = kernel.borrow().editor.syntax_manager.clone();
  let syntax_manager = lock!(syntax_manager);
  syntax_manager
    .loader()
    .set_treesitter_parser_lib_path(Path::new(&value).to_path_buf());
  Ok(())
}

/// Installs the `opt_*` built-ins on `internal`.
pub fn install<'js>(ctx: &Ctx<'js>, internal: &Object<'js>) -> JsResult<()> {
  set_function_to(ctx, internal, "opt_get_wrap", get_wrap)?;
  set_function_to(ctx, internal, "opt_set_wrap", set_wrap)?;
  set_function_to(ctx, internal, "opt_get_line_break", get_line_break)?;
  set_function_to(ctx, internal, "opt_set_line_break", set_line_break)?;
  set_function_to(ctx, internal, "opt_get_tab_stop", get_tab_stop)?;
  set_function_to(ctx, internal, "opt_set_tab_stop", set_tab_stop)?;
  set_function_to(ctx, internal, "opt_get_expand_tab", get_expand_tab)?;
  set_function_to(ctx, internal, "opt_set_expand_tab", set_expand_tab)?;
  set_function_to(ctx, internal, "opt_get_shift_width", get_shift_width)?;
  set_function_to(ctx, internal, "opt_set_shift_width", set_shift_width)?;
  set_function_to(ctx, internal, "opt_get_file_encoding", get_file_encoding)?;
  set_function_to(ctx, internal, "opt_set_file_encoding", set_file_encoding)?;
  set_function_to(ctx, internal, "opt_get_file_format", get_file_format)?;
  set_function_to(ctx, internal, "opt_set_file_format", set_file_format)?;
  set_function_to(
    ctx,
    internal,
    "opt_get_fix_end_of_line",
    get_fix_end_of_line,
  )?;
  set_function_to(
    ctx,
    internal,
    "opt_set_fix_end_of_line",
    set_fix_end_of_line,
  )?;
  set_function_to(
    ctx,
    internal,
    "opt_get_syntax_parser_lib_path",
    get_syntax_parser_lib_path,
  )?;
  set_function_to(
    ctx,
    internal,
    "opt_set_syntax_parser_lib_path",
    set_syntax_parser_lib_path,
  )?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use crate::js::binding::install_bindings;
  use crate::js::runtime;
  use crate::js::runtime::JsKernel;
  use crate::js::runtime::JsRuntimeOptions;
  use crate::js::runtime::set_state;
  use crate::prelude::*;
  use crate::tests::js::make_editor;
  use crate::tests::log::init as test_log_init;
  use rquickjs::Context;
  use rquickjs::Runtime;

  fn with_kernel<R>(
    f: impl FnOnce(rquickjs::Ctx<'_>, runtime::JsKernelRc) -> R,
  ) -> R {
    let editor = make_editor();
    let (master_tx, _master_rx) = tokio::sync::mpsc::unbounded_channel();
    let kernel = JsKernel::to_rc(JsKernel {
      options: JsRuntimeOptions::default(),
      master_tx,
      editor,
    });
    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();
    ctx.with(move |ctx| {
      set_state(&ctx, kernel.clone()).unwrap();
      install_bindings(&ctx).unwrap();
      f(ctx, kernel)
    })
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn wrap_round_trips_through_the_binding() {
    test_log_init();
    with_kernel(|ctx, kernel| {
      let before: bool = ctx
        .eval("__InternalRsvimGlobalObject.opt_get_wrap()")
        .unwrap();
      assert_eq!(
        before,
        lock!(kernel.borrow().editor.tree.clone())
          .global_local_options()
          .wrap()
      );

      ctx
        .eval::<(), _>("__InternalRsvimGlobalObject.opt_set_wrap(true)")
        .unwrap();
      assert!(
        lock!(kernel.borrow().editor.tree.clone())
          .global_local_options()
          .wrap()
      );
      let after: bool = ctx
        .eval("__InternalRsvimGlobalObject.opt_get_wrap()")
        .unwrap();
      assert!(after);
    });
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn tab_stop_coerces_a_number() {
    test_log_init();
    with_kernel(|ctx, kernel| {
      ctx
        .eval::<(), _>("__InternalRsvimGlobalObject.opt_set_tab_stop(6)")
        .unwrap();
      let value = lock!(kernel.borrow().editor.buffer_manager.clone())
        .global_local_options()
        .tab_stop();
      assert_eq!(value, 6);

      let back: i32 = ctx
        .eval("__InternalRsvimGlobalObject.opt_get_tab_stop()")
        .unwrap();
      assert_eq!(back, 6);
    });
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn file_encoding_rejects_an_unknown_name() {
    test_log_init();
    with_kernel(|ctx, _kernel| {
      let result = ctx.eval::<(), _>(
        "__InternalRsvimGlobalObject.opt_set_file_encoding('not-an-encoding')",
      );
      // The v8 backend paniced here (`try_from(...).unwrap()`); the rqjs port
      // reports a range error instead, which is strictly better than aborting
      // the whole editor on a typo in a config file.
      assert!(result.is_err());
    });
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn file_format_round_trips() {
    test_log_init();
    with_kernel(|ctx, kernel| {
      let format: String = ctx
        .eval("__InternalRsvimGlobalObject.opt_get_file_format()")
        .unwrap();
      assert_eq!(
        format,
        lock!(kernel.borrow().editor.buffer_manager.clone())
          .global_local_options()
          .file_format()
          .to_string()
      );
    });
  }
}
