//! Syntax APIs.

pub mod load;
pub mod metadata;

use crate::js;
use crate::js::JsRuntime;
use crate::js::binding;
use crate::js::converter::*;
use crate::js::pending;
use crate::prelude::*;
use crate::syntax;
use crate::syntax::SyntaxLoadGrammarRequest;
use compact_str::ToCompactString;
pub use load::SynLoadTreeSitterParserFuture;
pub use load::SynLoadTreeSitterParserOptions;
pub use metadata::SynTreeSitterParserMetadata;

/// Javascript `loadParserSync` API.
pub fn load_parser_sync<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  debug_assert!(args.length() == 1);
  let options = SynLoadTreeSitterParserOptions::from_v8(scope, args.get(0));
  trace!("syn.loadParserSync:{:?}", options);

  let state_rc = JsRuntime::state(scope);
  let state = state_rc.borrow();

  match Path::new(&options.grammar_path).absolutize() {
    Ok(grammar_path) => {
      let load_req = SyntaxLoadGrammarRequest {
        grammar_path: grammar_path.to_path_buf(),
      };
      match syntax::load_syntax_grammar(
        state.editor.syntax_manager.clone(),
        &load_req,
      ) {
        Ok(metainfo) => {
          let parser_names = metainfo
            .grammars
            .iter()
            .map(|grammar| grammar.name.to_string())
            .collect::<Vec<String>>();
          trace!("syn.loadParserSync result:{:?}", parser_names);
          let parser_names = parser_names.to_v8(scope);
          rv.set(parser_names);
        }
        Err(e) => {
          binding::throw_exception(scope, &e);
        }
      }
    }
    Err(_e) => {
      let e = TheErr::TreeSitterParserNotFound(options.grammar_path.clone());
      binding::throw_exception(scope, &e);
    }
  }
}

/// Javascript `loadParser` API.
pub fn load_treesitter_parser<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  debug_assert!(args.length() == 1);
  let options = SynLoadTreeSitterParserOptions::from_v8(scope, args.get(0));
  trace!("syn.loadParser:{:?}", options);

  let promise_resolver = v8::PromiseResolver::new(scope).unwrap();
  let promise = promise_resolver.get_promise(scope);

  let state_rc = JsRuntime::state(scope);
  let load_cb = {
    let promise = v8::Global::new(scope, promise_resolver);
    let state_rc = state_rc.clone();
    move |maybe_result: Option<TheResult<Vec<u8>>>| {
      let fut = SynLoadTreeSitterParserFuture {
        promise: promise.clone(),
        maybe_result,
      };
      let mut state = state_rc.borrow_mut();
      state.pending_futures.push(Box::new(fut));
    }
  };

  let mut state = state_rc.borrow_mut();
  let task_id = js::TaskId::next();
  let grammar_path = Path::new(&options.grammar_path);
  pending::create_syn_load_parser(
    &mut state,
    task_id,
    grammar_path,
    Box::new(load_cb),
  );

  rv.set(promise.into());
}

/// Javascript `listParsers` API.
pub fn list_parsers<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  debug_assert!(args.length() == 0);
  trace!("syn.listParsers");

  let state_rc = JsRuntime::state(scope);
  let state = state_rc.borrow();

  let syntax_manager = state.editor.syntax_manager.clone();
  let parser_names = lock!(syntax_manager).list_grammar_names();
  trace!("syn.listParsers result:{:?}", parser_names);
  let parser_names = parser_names.to_v8(scope);
  rv.set(parser_names);
}

/// Javascript `getParserMetadata` API.
pub fn get_parser_metadata<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  debug_assert!(args.length() == 1);
  let parser_name = args.get(0).to_rust_string_lossy(scope);
  trace!("syn.getParserMetadata:{:?}", parser_name);

  let state_rc = JsRuntime::state(scope);
  let state = state_rc.borrow();

  let syntax_manager = state.editor.syntax_manager.clone();
  let syntax_manager = lock!(syntax_manager);
  match syntax_manager.get_metadata(&parser_name) {
    Some(metadata) => {
      trace!("syn.getParserMetadata result:{:?}", metadata);
      let metadata1 = SynTreeSitterParserMetadata {
        name: metadata.name.clone(),
        camelcase: metadata.camelcase.clone(),
        scope: metadata.scope.clone(),
        path: metadata.path.to_string_lossy().to_compact_string(),
        file_types: metadata.file_types.clone(),
        highlights_path: metadata
          .highlights_path
          .as_ref()
          .map(|p| p.to_string_lossy().to_compact_string()),
        highlights_query: metadata.highlights_query.clone(),
        tags_path: metadata
          .tags_path
          .as_ref()
          .map(|p| p.to_string_lossy().to_compact_string()),
        tags_query: metadata.tags_query.clone(),
        injections_path: metadata
          .injections_path
          .as_ref()
          .map(|p| p.to_string_lossy().to_compact_string()),
        injections_query: metadata.injections_query.clone(),
        injection_regex: metadata.injection_regex.clone(),
      };
      let metadata1 = metadata1.to_v8(scope);
      rv.set(metadata1);
    }
    None => {
      rv.set_undefined();
    }
  }
}
