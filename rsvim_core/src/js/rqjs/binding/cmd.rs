//! APIs for the `cmd` module.

use crate::js::cmdline::send_cmdline_message;
use crate::js::rqjs::binding::set_function_to;
use crate::js::rqjs::get_state;
use rquickjs::Ctx;
use rquickjs::Exception;
use rquickjs::Object;
use rquickjs::Result as JsResult;

/// `cmd.echo(message)`.
///
/// The javascript wrapper in `rsvim_core/js/cmd.min.js` has already checked that
/// the argument is not nil, so this takes an owned `String` and lets `rquickjs`
/// coerce whatever it was given.
pub fn echo<'js>(ctx: Ctx<'js>, message: String) -> JsResult<()> {
  let kernel = get_state(&ctx).ok_or_else(|| {
    Exception::throw_message(&ctx, "js kernel is not initialised")
  })?;
  send_cmdline_message(&kernel.borrow().editor, message);
  Ok(())
}

/// Installs the `cmd_*` built-ins on `internal`.
///
/// `cmd_create` / `cmd_list` / `cmd_get` / `cmd_remove` are not here yet: all
/// four carry a [`CommandDefinition`] across the boundary, and its callback is
/// still an `Rc<v8::Global<v8::Function>>`. They land once that type is engine
/// neutral, which is the first thing Step 4 does.
///
/// [`CommandDefinition`]: crate::js::command::def::CommandDefinition
pub fn install<'js>(ctx: &Ctx<'js>, internal: &Object<'js>) -> JsResult<()> {
  set_function_to(ctx, internal, "cmd_echo", echo)?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use crate::js::rqjs::binding::install_bindings;
  use crate::js::rqjs::runtime;
  use crate::js::rqjs::runtime::JsKernel;
  use crate::js::rqjs::runtime::JsRuntimeOptions;
  use crate::js::rqjs::runtime::set_state;
  use crate::prelude::*;
  use crate::tests::js::make_editor;
  use crate::tests::log::init as test_log_init;
  use rquickjs::AsyncContext;
  use rquickjs::AsyncRuntime;

  fn with_kernel<R>(
    f: impl FnOnce(rquickjs::Ctx<'_>, runtime::JsKernelRc) -> R,
  ) -> R {
    let editor = make_editor();
    let (master_tx, _master_rx) = tokio::sync::mpsc::unbounded_channel();
    let kernel = JsKernel::to_rc(JsKernel {
      options: JsRuntimeOptions::default(),
      master_tx,
      editor: editor.clone(),
    });
    let rt = AsyncRuntime::new().unwrap();
    let ctx = futures::executor::block_on(AsyncContext::full(&rt)).unwrap();
    futures::executor::block_on(ctx.with(move |ctx| {
      set_state(&ctx, kernel.clone()).unwrap();
      install_bindings(&ctx).unwrap();
      f(ctx, kernel)
    }))
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn echo_reaches_the_message_history() {
    test_log_init();
    with_kernel(|ctx, kernel| {
      ctx
        .eval::<(), _>("__InternalRsvimGlobalObject.cmd_echo('hello')")
        .unwrap();
      let history = lock!(kernel.borrow().editor.cmdline_text)
        .message_history()
        .iter()
        .cloned()
        .collect::<Vec<String>>();
      assert_eq!(history, vec!["hello".to_string()]);
    });
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn echo_through_the_javascript_wrapper() {
    test_log_init();
    with_kernel(|ctx, kernel| {
      ctx
        .eval::<(), _>(
          "const cmd = __InternalRsvimGlobalObject; cmd.cmd_echo('wrapped');",
        )
        .unwrap();
      let history = lock!(kernel.borrow().editor.cmdline_text)
        .message_history()
        .iter()
        .cloned()
        .collect::<Vec<String>>();
      assert_eq!(history, vec!["wrapped".to_string()]);
    });
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn echo_without_a_kernel_is_a_javascript_error() {
    test_log_init();
    let rt = AsyncRuntime::new().unwrap();
    let ctx = futures::executor::block_on(AsyncContext::full(&rt)).unwrap();
    futures::executor::block_on(ctx.with(|ctx| {
      // Bindings without userdata: the call must raise, not panic.
      install_bindings(&ctx).unwrap();
      let result =
        ctx.eval::<(), _>("__InternalRsvimGlobalObject.cmd_echo('x')");
      assert!(result.is_err());
    }));
  }
}
