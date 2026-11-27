use crate::lock;
use crate::tests::js::with_kernel;
use crate::tests::log::init as test_log_init;

#[test]
#[cfg_attr(miri, ignore)]
fn text_encoder_is_installed() {
  test_log_init();
  with_kernel(|ctx, _kernel| {
    let ty: String = ctx.eval("typeof TextEncoder").unwrap();
    assert_eq!(ty, "function");
    let ty: String = ctx.eval("typeof TextDecoder").unwrap();
    assert_eq!(ty, "function");
  });
}

#[test]
#[cfg_attr(miri, ignore)]
fn encode_returns_utf8_bytes() {
  test_log_init();
  with_kernel(|ctx, _kernel| {
    let bytes: Vec<u8> = ctx
      .eval("Array.from(new TextEncoder().encode('Hi'))")
      .unwrap();
    assert_eq!(bytes, vec![72, 105]);
  });
}

#[test]
#[cfg_attr(miri, ignore)]
fn decode_roundtrips_multibyte_text() {
  test_log_init();
  with_kernel(|ctx, _kernel| {
    // Three-byte characters prove the UTF-16 path, not just ASCII.
    let text: String = ctx
      .eval(
        "const bytes = new TextEncoder().encode('日本語');
           new TextDecoder('utf-8').decode(bytes);",
      )
      .unwrap();
    assert_eq!(text, "日本語");
  });
}

#[test]
#[cfg_attr(miri, ignore)]
fn streaming_decoder_accepts_split_input() {
  test_log_init();
  with_kernel(|ctx, _kernel| {
    let text: String = ctx
      .eval(
        "const dec = new TextDecoder('utf-8');
           const a = new TextEncoder().encode('ab');
           const b = new TextEncoder().encode('cd');
           dec.decode(a, {stream: true}) + dec.decode(b);",
      )
      .unwrap();
    assert_eq!(text, "abcd");
  });
}

#[test]
#[cfg_attr(miri, ignore)]
fn queue_microtask_runs_after_the_script() {
  test_log_init();
  with_kernel(|ctx, _kernel| {
    ctx
      .eval::<(), _>(
        "globalThis.__order = [];
         queueMicrotask(() => __order.push('micro'));
         __order.push('sync');",
      )
      .unwrap();
    while ctx.execute_pending_job() {}
    let order: String = ctx.eval("globalThis.__order.join(',')").unwrap();
    assert_eq!(order, "sync,micro");
  });
}

#[test]
#[cfg_attr(miri, ignore)]
fn report_error_reaches_the_cmdline() {
  test_log_init();
  with_kernel(|ctx, kernel| {
    ctx.eval::<(), _>("reportError(new Error('boom'))").unwrap();
    let history = {
      let editor = kernel.borrow().editor.clone();
      let cmdline = lock!(editor.cmdline_text);
      cmdline
        .message_history()
        .iter()
        .cloned()
        .collect::<Vec<String>>()
    };
    assert!(
      history.iter().any(|m| m.contains("boom")),
      "expected the message to mention the error, got {history:?}"
    );
  });
}
