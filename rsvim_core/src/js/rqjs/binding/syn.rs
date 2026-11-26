//! Syntax APIs.
//!
//! Only the synchronous half lives here so far: `listParsers` and
//! `getParserMetadata`. `loadParserSync` and the promise-returning `loadParser`
//! need the pending-task machinery, which lands with the async bindings.

use crate::js::rqjs::binding::set_function_to;
use crate::js::rqjs::get_state;
use crate::prelude::*;
use rquickjs::Ctx;
use rquickjs::Exception;
use rquickjs::Object;
use rquickjs::Result as JsResult;

/// Builds the javascript object `getParserMetadata` hands back.
///
/// The v8 backend gets this for free from `#[derive(ToV8)]`, which turns every
/// field into a lowerCamelCase property. `rquickjs` has no such derive for an
/// arbitrary struct, and the field set is small enough that spelling it out is
/// cheaper than a macro.
fn metadata_to_object<'js>(
  ctx: &Ctx<'js>,
  metadata: &crate::syntax::SyntaxTreeSitterGrammarMetadata,
) -> JsResult<Object<'js>> {
  let object = Object::new(ctx.clone())?;
  object.set("name", metadata.name.to_string())?;
  object.set("camelcase", metadata.camelcase.to_string())?;
  object.set("scope", metadata.scope.to_string())?;
  object.set("path", metadata.path.to_string_lossy().to_string())?;
  let file_types = metadata
    .file_types
    .iter()
    .map(|t| t.to_string())
    .collect::<Vec<String>>();
  object.set("fileTypes", file_types)?;
  object.set(
    "highlightsPath",
    metadata
      .highlights_path
      .as_ref()
      .map(|p| p.to_string_lossy().to_string()),
  )?;
  object.set("highlightsQuery", metadata.highlights_query.clone())?;
  object.set(
    "tagsPath",
    metadata
      .tags_path
      .as_ref()
      .map(|p| p.to_string_lossy().to_string()),
  )?;
  object.set("tagsQuery", metadata.tags_query.clone())?;
  object.set(
    "injectionsPath",
    metadata
      .injections_path
      .as_ref()
      .map(|p| p.to_string_lossy().to_string()),
  )?;
  object.set("injectionsQuery", metadata.injections_query.clone())?;
  object.set("injectionRegex", metadata.injection_regex.clone())?;
  Ok(object)
}

/// `syn.listParsers()`.
pub fn list_parsers<'js>(ctx: Ctx<'js>) -> JsResult<Vec<String>> {
  trace!("syn.listParsers");
  let kernel = get_state(&ctx).ok_or_else(|| {
    Exception::throw_message(&ctx, "js kernel is not initialised")
  })?;
  let syntax_manager = kernel.borrow().editor.syntax_manager.clone();
  let parser_names = lock!(syntax_manager).list_grammar_names();
  trace!("syn.listParsers result:{:?}", parser_names);
  Ok(parser_names.into_iter().map(|n| n.to_string()).collect())
}

/// `syn.getParserMetadata(parserName)`.
///
/// Returns `undefined` when the parser is unknown, matching the v8 backend.
pub fn get_parser_metadata<'js>(
  ctx: Ctx<'js>,
  parser_name: String,
) -> JsResult<Option<Object<'js>>> {
  trace!("syn.getParserMetadata:{:?}", parser_name);
  let kernel = get_state(&ctx).ok_or_else(|| {
    Exception::throw_message(&ctx, "js kernel is not initialised")
  })?;
  let syntax_manager = kernel.borrow().editor.syntax_manager.clone();
  let syntax_manager = lock!(syntax_manager);
  match syntax_manager.get_metadata(&parser_name) {
    Some(metadata) => {
      trace!("syn.getParserMetadata result:{:?}", metadata);
      Ok(Some(metadata_to_object(&ctx, metadata)?))
    }
    None => Ok(None),
  }
}

/// Installs the `syn_*` built-ins on `internal`.
pub fn install<'js>(ctx: &Ctx<'js>, internal: &Object<'js>) -> JsResult<()> {
  set_function_to(ctx, internal, "syn_list_parsers", list_parsers)?;
  set_function_to(
    ctx,
    internal,
    "syn_get_parser_metadata",
    get_parser_metadata,
  )?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use crate::js::rqjs::binding::install_bindings;
  use crate::js::rqjs::runtime;
  use crate::js::rqjs::runtime::JsKernel;
  use crate::js::rqjs::runtime::JsRuntimeOptions;
  use crate::js::rqjs::runtime::set_state;
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
      editor,
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
  fn list_parsers_is_an_array() {
    test_log_init();
    with_kernel(|ctx, _kernel| {
      let is_array: bool = ctx
        .eval("Array.isArray(__InternalRsvimGlobalObject.syn_list_parsers())")
        .unwrap();
      assert!(is_array);
    });
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn unknown_parser_metadata_is_undefined() {
    test_log_init();
    with_kernel(|ctx, _kernel| {
      let kind: String = ctx
        .eval(
          "typeof __InternalRsvimGlobalObject.syn_get_parser_metadata('nope')",
        )
        .unwrap();
      assert_eq!(kind, "undefined");
    });
  }
}
