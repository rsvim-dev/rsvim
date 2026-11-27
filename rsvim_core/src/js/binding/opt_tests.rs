use crate::prelude::*;
use crate::tests::js::with_kernel;
use crate::tests::log::init as test_log_init;

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
