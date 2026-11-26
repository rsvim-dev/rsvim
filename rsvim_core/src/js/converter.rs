//! Conversions between Rust and javascript values.
//!
//! Most of the v8 `converter.rs` (510 lines) has no counterpart here: `rquickjs`
//! already implements `IntoJs` / `FromJs` for `String`, the numeric types,
//! `bool`, `Option<T>`, `Vec<T>` and the object/array/`ArrayBuffer` handles. What
//! is left is the handful of rsvim types that need a decision about how they
//! cross the boundary.
//!
//! rsvim's own string type is `CompactString`, which is a foreign type, so the
//! orpan rule forbids `impl IntoJs for CompactString` here. Plain functions do
//! the job instead, and the bindings are expected to declare `String` /
//! `Coerced<String>` parameters and convert at the call site.

use compact_str::CompactString;
use rquickjs::ArrayBuffer;
use rquickjs::Ctx;
use rquickjs::FromJs;
use rquickjs::Result as JsResult;
use rquickjs::Value;

/// Borrows the bytes of an `ArrayBuffer` as a `Vec<u8>`.
///
/// This copies. `ArrayBuffer::as_bytes` hands out a borrowed slice, but the
/// bytes are about to be handed to a background task that will outlive the
/// scope, so the copy is mandatory rather than wasteful.
pub fn array_buffer_to_vec<'js>(
  ctx: &Ctx<'js>,
  value: Value<'js>,
) -> JsResult<Vec<u8>> {
  let buffer = ArrayBuffer::from_js(ctx, value)?;
  // SAFETY: the buffer is not shared and lives for the whole call.
  let bytes = unsafe { buffer.as_bytes() }.ok_or_else(|| {
    rquickjs::Exception::throw_type(
      ctx,
      "expected an ArrayBuffer, not a SharedArrayBuffer",
    )
  })?;
  Ok(bytes.to_vec())
}

/// Copies `bytes` into a fresh `ArrayBuffer`.
pub fn vec_to_array_buffer<'js>(
  ctx: Ctx<'js>,
  bytes: Vec<u8>,
) -> JsResult<ArrayBuffer<'js>> {
  ArrayBuffer::new_copy(ctx, bytes)
}

/// `CompactString` from a javascript value.
pub fn compact_string_from_js<'js>(
  ctx: &Ctx<'js>,
  value: Value<'js>,
) -> JsResult<CompactString> {
  let s = String::from_js(ctx, value)?;
  Ok(CompactString::from(s))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::tests::log::init as test_log_init;
  use rquickjs::Context;
  use rquickjs::IntoJs;
  use rquickjs::Runtime;

  fn with_ctx<R>(f: impl FnOnce(Ctx<'_>) -> R) -> R {
    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();
    ctx.with(f)
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn array_buffer_round_trip() {
    test_log_init();
    with_ctx(|ctx| {
      let bytes = b"rsvim".to_vec();
      let buffer = vec_to_array_buffer(ctx.clone(), bytes.clone()).unwrap();
      let back = array_buffer_to_vec(&ctx, buffer.into_value()).unwrap();
      assert_eq!(back, bytes);
    });
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn compact_string_comes_back() {
    test_log_init();
    with_ctx(|ctx| {
      let js = "rsvim".into_js(&ctx).unwrap();
      let back = compact_string_from_js(&ctx, js).unwrap();
      assert_eq!(back, CompactString::from("rsvim"));
    });
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn non_array_buffer_is_rejected() {
    test_log_init();
    with_ctx(|ctx| {
      let value = 42i32.into_js(&ctx).unwrap();
      assert!(array_buffer_to_vec(&ctx, value).is_err());
    });
  }
}
