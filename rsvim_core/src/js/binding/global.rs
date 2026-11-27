//! `TextEncoder`, `TextDecoder`, `queueMicrotask` and `reportError`.
//!
//! The timer half of the web globals (`setTimeout`/`setInterval`) belongs to the
//! async step: a timer has to park a callback and come back to it later, which
//! needs the completion channel that step introduces.

use crate::js::binding::set_function_to;
use crate::js::converter::array_buffer_to_vec;
use crate::js::converter::copy_into_array_buffer;
use crate::js::err::JsError;
use crate::js::resource::Resource;
use crate::js::resource::ResourceId;
use crate::js::runtime::kernel;
use crate::prelude::*;
use encoding_rs::CoderResult;
use encoding_rs::Decoder;
use encoding_rs::DecoderResult;
use encoding_rs::Encoding;
use rquickjs::ArrayBuffer;
use rquickjs::Coerced;
use rquickjs::Ctx;
use rquickjs::Exception;
use rquickjs::Function;
use rquickjs::Object;
use rquickjs::Result as JsResult;
use rquickjs::String as JsString;
use rquickjs::Value;

#[cfg(test)]
mod microtask_tests;
#[cfg(test)]
mod text_encoder_tests;

/// `TextEncoder.encode(string)`.
///
/// Returns a `Uint8Array`, which javascript sees as an array of bytes — not an
/// `ArrayBuffer`. `00__web.min.js` passes the result on to `fs.write` unchanged.
pub fn encoding_encode<'js>(
  ctx: Ctx<'js>,
  payload: Coerced<String>,
) -> JsResult<rquickjs::TypedArray<'js, u8>> {
  let bytes = payload.0.into_bytes();
  rquickjs::TypedArray::new(ctx, bytes)
}

/// `TextEncoder.encodeInto(string, uint8array)`.
///
/// Writes as many bytes as fit and reports both counts, per the encoding
/// standard.
pub fn encoding_encode_into<'js>(
  ctx: Ctx<'js>,
  payload: Coerced<String>,
  buffer: ArrayBuffer<'js>,
) -> JsResult<Object<'js>> {
  let bytes = payload.0.into_bytes();
  let n = copy_into_array_buffer(&buffer, &bytes).ok_or_else(|| {
    Exception::throw_type(&ctx, "encodeInto: detached buffer")
  })?;

  let result = Object::new(ctx.clone())?;
  result.set("read", n)?;
  result.set("written", n)?;
  Ok(result)
}

/// `TextEncoder`'s `checkEncodingLabel` helper.
pub fn encoding_check_encoding_label(
  _ctx: Ctx<'_>,
  label: Coerced<String>,
) -> JsResult<bool> {
  Ok(Encoding::for_label(label.0.as_bytes()).is_some())
}

fn create_decoder(label: &str, ignore_bom: bool) -> JsResult<Decoder> {
  let encoding = Encoding::for_label(label.as_bytes()).ok_or_else(|| {
    rquickjs::Error::new_from_js_message(
      "string",
      "encoding label",
      "unknown encoding",
    )
  })?;
  Ok(if ignore_bom {
    encoding.new_decoder_without_bom_handling()
  } else {
    encoding.new_decoder_with_bom_removal()
  })
}

/// Shared body of the two decode entry points. Mirrors `decode_impl` in the v8
/// backend: fatal decoders reject malformed input, lenient ones substitute.
fn decode<'js>(
  ctx: &Ctx<'js>,
  decoder: &mut Decoder,
  data: &[u8],
  fatal: bool,
  stream: bool,
) -> JsResult<JsString<'js>> {
  let Some(max_len) = decoder.max_utf16_buffer_length(data.len()) else {
    return Err(Exception::throw_range(ctx, "input too large"));
  };
  let mut out = vec![0u16; max_len];

  if fatal {
    let (result, _read, written) =
      decoder.decode_to_utf16_without_replacement(data, &mut out, !stream);
    match result {
      DecoderResult::InputEmpty => {}
      DecoderResult::OutputFull => {
        return Err(Exception::throw_type(ctx, "output buffer too small"));
      }
      DecoderResult::Malformed(_, _) => {
        return Err(Exception::throw_type(ctx, "malformed input"));
      }
    }
    out.truncate(written);
  } else {
    let (result, _read, written, _had_errors) =
      decoder.decode_to_utf16(data, &mut out, !stream);
    match result {
      CoderResult::InputEmpty => {}
      CoderResult::OutputFull => {
        return Err(Exception::throw_type(ctx, "output buffer too small"));
      }
    }
    out.truncate(written);
  }

  // `String::from_str` only takes UTF-8, and the decoder produces UTF-16.
  // Round-tripping through `String::from_utf16` is safe because the decoder's
  // output is always valid UTF-16.
  let text = std::string::String::from_utf16(&out).map_err(|_| {
    Exception::throw_type(ctx, "decoder produced invalid utf-16")
  })?;
  JsString::from_str(ctx.clone(), &text)
}

/// `TextDecoder.decode(buffer, label, fatal, ignoreBom)` — one-shot decoding.
pub fn encoding_decode_single<'js>(
  ctx: Ctx<'js>,
  data: ArrayBuffer<'js>,
  label: Coerced<String>,
  fatal: bool,
  ignore_bom: bool,
) -> JsResult<JsString<'js>> {
  let bytes = array_buffer_to_vec(&ctx, data.into_value())?;
  let mut decoder = create_decoder(&label.0, ignore_bom)?;
  decode(&ctx, &mut decoder, &bytes, fatal, false)
}

/// `new TextDecoder().decode(..., {stream: true})` — allocates a decoder and
/// returns its resource id.
pub fn encoding_create_stream_decoder(
  ctx: Ctx<'_>,
  label: Coerced<String>,
  ignore_bom: bool,
) -> JsResult<i32> {
  let decoder = create_decoder(&label.0, ignore_bom)?;
  let kernel = kernel(&ctx)?;
  let table = kernel.borrow().editor.resource_table.clone();
  let rid = lock!(table).add_text_decoder(decoder);
  Ok(rid.into())
}

/// Continues a streaming decode with the decoder allocated above.
pub fn encoding_decode_stream<'js>(
  ctx: Ctx<'js>,
  data: ArrayBuffer<'js>,
  decoder_rid: Coerced<i32>,
  fatal: bool,
  stream: bool,
) -> JsResult<JsString<'js>> {
  let bytes = array_buffer_to_vec(&ctx, data.into_value())?;
  let rid = ResourceId::from(decoder_rid.0);
  let kernel = kernel(&ctx)?;
  let table = kernel.borrow().editor.resource_table.clone();
  let handle = lock!(table).get(&rid).cloned().ok_or_else(|| {
    Exception::throw_type(&ctx, "unknown text decoder handle")
  })?;
  match handle {
    Resource::TextDecoder(res) => {
      let decoder = res.data();
      let mut decoder = lock!(decoder);
      decode(&ctx, &mut decoder, &bytes, fatal, stream)
    }
    _ => Err(Exception::throw_type(&ctx, "handle is not a text decoder")),
  }
}

/// Frees a streaming decoder.
pub fn encoding_close_stream_decoder(
  ctx: Ctx<'_>,
  decoder_rid: Coerced<i32>,
) -> JsResult<()> {
  let rid = ResourceId::from(decoder_rid.0);
  let kernel = kernel(&ctx)?;
  let table = kernel.borrow().editor.resource_table.clone();
  let _ = lock!(table).remove(&rid);
  Ok(())
}

/// `queueMicrotask(callback)`.
///
/// `Function::defer` is exactly this: it pushes the call onto QuickJS's job
/// queue, which is the microtask queue. v8 needed the context's
/// `MicrotaskQueue` handle instead.
pub fn queue_microtask<'js>(
  _ctx: Ctx<'js>,
  callback: Function<'js>,
) -> JsResult<()> {
  callback.defer(())
}

/// `reportError(exception)`.
///
/// There is no rejection tracker in play here — this is the explicit path
/// `00__web.min.js` uses when a callback throws. The error is shown the way
/// every other uncaught error is, see [`JsError::from_exception`].
pub fn report_error<'js>(ctx: Ctx<'js>, error: Value<'js>) -> JsResult<()> {
  let kernel = kernel(&ctx)?;
  let error = JsError::from_js_value(&ctx, error);
  crate::js::err::report_js_error(&kernel.borrow().editor, error);
  Ok(())
}

/// Installs the web globals that do not need the event loop.
pub fn install<'js>(ctx: &Ctx<'js>, internal: &Object<'js>) -> JsResult<()> {
  set_function_to(ctx, internal, "global_encoding_encode", encoding_encode)?;
  set_function_to(
    ctx,
    internal,
    "global_encoding_encode_into",
    encoding_encode_into,
  )?;
  set_function_to(
    ctx,
    internal,
    "global_encoding_check_encoding_label",
    encoding_check_encoding_label,
  )?;
  set_function_to(
    ctx,
    internal,
    "global_encoding_decode_single",
    encoding_decode_single,
  )?;
  set_function_to(
    ctx,
    internal,
    "global_encoding_create_stream_decoder",
    encoding_create_stream_decoder,
  )?;
  set_function_to(
    ctx,
    internal,
    "global_encoding_decode_stream",
    encoding_decode_stream,
  )?;
  set_function_to(
    ctx,
    internal,
    "global_encoding_close_stream_decoder",
    encoding_close_stream_decoder,
  )?;
  set_function_to(ctx, internal, "global_queue_microtask", queue_microtask)?;
  set_function_to(ctx, internal, "global_report_error", report_error)?;
  Ok(())
}
