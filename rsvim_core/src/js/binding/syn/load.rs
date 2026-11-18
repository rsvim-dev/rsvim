//! Load tree-sitter grammar APIs.

use crate::js::JsFuture;
use crate::js::binding;
use crate::js::converter::*;
use crate::prelude::*;
use compact_str::CompactString;
use compact_str::ToCompactString;

#[derive(
  Debug,
  Clone,
  PartialEq,
  Eq,
  derive_builder::Builder,
  rsvim_macro::ToV8,
  rsvim_macro::FromV8,
)]
pub struct SynLoadTreeSitterParserOptions {
  #[builder(default = "".to_compact_string())]
  pub grammar_path: CompactString,
}

pub struct SynLoadTreeSitterParserFuture {
  pub promise: v8::Global<v8::PromiseResolver>,
  pub maybe_result: Option<TheResult<Vec<u8>>>,
}

impl JsFuture for SynLoadTreeSitterParserFuture {
  fn run(&mut self, scope: &mut v8::PinScope) {
    trace!("|SynLoadTreeSitterGrammarFuture|");

    let result = self.maybe_result.take().unwrap();

    // Handle when something goes wrong with it.
    if let Err(e) = result {
      let message = v8::String::new(scope, &e.to_string()).unwrap();
      let exception = v8::Exception::error(scope, message);
      binding::set_exception_code(scope, exception, &e);
      self.promise.open(scope).reject(scope, exception);
      return;
    }

    // Otherwise, get the result and deserialize it.
    let result = result.unwrap();

    // Deserialize bytes into a list of parser names.
    let parser_names = postcard::from_bytes::<Vec<String>>(&result).unwrap();
    let parser_names = parser_names.to_v8(scope);

    self
      .promise
      .open(scope)
      .resolve(scope, parser_names)
      .unwrap();
  }
}
