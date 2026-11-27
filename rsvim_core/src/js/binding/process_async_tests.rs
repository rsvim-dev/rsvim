use crate::tests::js::make_js_host;
use crate::tests::log::init as test_log_init;

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn waiting_on_an_unknown_child_rejects_the_promise() {
  test_log_init();
  let mut js = make_js_host();

  let outcome = js
    .settle("__InternalRsvimGlobalObject.process_wait_child(9999)")
    .await;

  let message = outcome.expect_err("an unknown child must reject");
  assert!(
    message.contains("not found") || message.contains("9999"),
    "the rejection must name the missing child, got: {message}"
  );
}
