//! Syntax APIs.
//!
//! `syn.listParsers`, `syn.getParserMetadata`, `syn.loadParserSync` and
//! `syn.loadParser`.

use crate::js::binding::set_function_to;
use crate::js::ops::OpKind;
use crate::js::ops::spawn_simple_op;
use crate::js::runtime::kernel;
use crate::prelude::*;
use crate::syntax;
use crate::syntax::SyntaxLoadGrammarRequest;
use crate::syntax::SyntaxTreeSitterGrammarRepository;
use rquickjs::Ctx;
use rquickjs::Exception;
use rquickjs::Object;
use rquickjs::Promise;
use rquickjs::Result as JsResult;
use rquickjs::Value;
use rquickjs::function::Opt;

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
  object.set("highlightsQuery", metadata.highlights_query.as_deref())?;
  object.set(
    "tagsPath",
    metadata
      .tags_path
      .as_ref()
      .map(|p| p.to_string_lossy().to_string()),
  )?;
  object.set("tagsQuery", metadata.tags_query.as_deref())?;
  object.set(
    "injectionsPath",
    metadata
      .injections_path
      .as_ref()
      .map(|p| p.to_string_lossy().to_string()),
  )?;
  object.set("injectionsQuery", metadata.injections_query.as_deref())?;
  object.set("injectionRegex", metadata.injection_regex.clone())?;
  Ok(object)
}

/// `syn.listParsers()`.
pub fn list_parsers<'js>(ctx: Ctx<'js>) -> JsResult<Vec<String>> {
  trace!("syn.listParsers");
  let kernel = kernel(&ctx)?;
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
  let kernel = kernel(&ctx)?;
  let syntax_manager = kernel.borrow().editor.syntax_manager.clone();
  let syntax_manager = lock!(syntax_manager);
  let metadata = syntax_manager.get_metadata(&parser_name);
  trace!("syn.getParserMetadata result:{:?}", metadata);
  metadata.map(|m| metadata_to_object(&ctx, m)).transpose()
}

/// Reads `{grammarPath}` and absolutizes it.
///
/// `syn.min.js` defaults the whole object, so a missing argument only happens
/// for a caller that skips the wrapper; an empty path then fails to absolutize
/// and surfaces as `TreeSitterParserNotFound`, which is the right error.
fn grammar_request<'js>(
  ctx: &Ctx<'js>,
  options: Option<Value<'js>>,
) -> JsResult<SyntaxLoadGrammarRequest> {
  let grammar_path: String = options
    .and_then(|v| v.as_object().cloned())
    .and_then(|object| object.get("grammarPath").ok())
    .unwrap_or_default();
  trace!("syn.loadParser:{:?}", grammar_path);

  match Path::new(&grammar_path).absolutize() {
    Ok(path) => Ok(SyntaxLoadGrammarRequest {
      grammar_path: path.to_path_buf(),
    }),
    Err(_) => Err(Exception::throw_message(
      ctx,
      &TheErr::TreeSitterParserNotFound(grammar_path.into()).to_string(),
    )),
  }
}

/// The names of the parsers a grammar provides.
fn grammar_names(metainfo: &SyntaxTreeSitterGrammarRepository) -> Vec<String> {
  metainfo
    .grammars
    .iter()
    .map(|g| g.name.to_string())
    .collect()
}

/// `syn.loadParserSync(options)`.
///
/// Loads a tree-sitter grammar and returns the parser names it provides.
pub fn load_parser_sync<'js>(
  ctx: Ctx<'js>,
  options: Opt<Value<'js>>,
) -> JsResult<Vec<String>> {
  let req = grammar_request(&ctx, options.0)?;
  let syntax_manager = kernel(&ctx)?.borrow().editor.syntax_manager.clone();
  syntax::load_syntax_grammar(syntax_manager, &req)
    .map(|metainfo| grammar_names(&metainfo))
    .map_err(|e| Exception::throw_message(&ctx, &e.to_string()))
}

/// `syn.loadParser(options)` -> `Promise<string[]>`.
///
/// The grammar load is already a coroutine (`syntax::async_load_syntax_grammar`),
/// so unlike `fs.read` there is nothing to hand off to a blocking pool: the
/// future runs straight inside the operation.
pub fn load_parser<'js>(
  ctx: Ctx<'js>,
  options: Opt<Value<'js>>,
) -> JsResult<Promise<'js>> {
  let req = grammar_request(&ctx, options.0)?;
  let syntax_manager = kernel(&ctx)?.borrow().editor.syntax_manager.clone();

  spawn_simple_op(&ctx, "syn.loadParser", OpKind::Detached, async move {
    syntax::async_load_syntax_grammar(syntax_manager, &req)
      .await
      .map(|metainfo| grammar_names(&metainfo))
  })
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
  set_function_to(ctx, internal, "syn_load_parser_sync", load_parser_sync)?;
  set_function_to(ctx, internal, "syn_load_parser", load_parser)?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use crate::js::runtime;
  use crate::tests::js::make_js_host;
  use crate::tests::log::init as test_log_init;

  /// Runs `f` inside a fully installed host.
  fn with_kernel<R>(
    f: impl FnOnce(rquickjs::Ctx<'_>, runtime::JsKernelRc) -> R,
  ) -> R {
    let mut js = make_js_host();
    let kernel = js.host.kernel().clone();
    js.enter(move |ctx| f(ctx, kernel))
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

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn load_parser_returns_a_thenable() {
    test_log_init();
    with_kernel(|ctx, _kernel| {
      // The async variant has to hand javascript something to `await`; it is a
      // promise created by `spawn_op`.
      let is_thenable: bool = ctx
        .eval(
          r#"
          (() => {
            let p;
            try {
              p = __InternalRsvimGlobalObject.syn_load_parser({ grammarPath: "nope" });
            } catch (e) {
              return false;
            }
            return typeof p === "object" && p !== null && typeof p.then === "function";
          })()
          "#,
        )
        .unwrap();
      assert!(is_thenable, "syn_load_parser must return a promise");
    });
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn load_parser_sync_reports_an_empty_path() {
    test_log_init();
    with_kernel(|ctx, _kernel| {
      // An empty path cannot be absolutized, so the binding has to surface the
      // same `TreeSitterParserNotFound` the v8 backend throws.
      let message: String = ctx
        .eval(
          r#"
          (() => {
            try {
              __InternalRsvimGlobalObject.syn_load_parser_sync({});
              return "no error";
            } catch (e) {
              return String(e);
            }
          })()
          "#,
        )
        .unwrap();
      assert!(
        message.contains("Tree-sitter parser not found"),
        "got: {message}"
      );
    });
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn load_parser_sync_accepts_a_missing_argument() {
    test_log_init();
    with_kernel(|ctx, _kernel| {
      // `syn.min.js` defaults the options object, so both call shapes reach the
      // binding; neither may blow up with a parameter-count error.
      let kind: String = ctx
        .eval(
          r#"
          (() => {
            try {
              __InternalRsvimGlobalObject.syn_load_parser_sync();
              return "returned";
            } catch (e) {
              return "threw";
            }
          })()
          "#,
        )
        .unwrap();
      assert_eq!(kind, "threw");
    });
  }
}
