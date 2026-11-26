//! Ex command definition.

use crate::is_v8_func;
use crate::is_v8_str;
use crate::js::command::attr::*;
use crate::js::command::opt::*;
use crate::js::converter::*;
use compact_str::CompactString;
use compact_str::ToCompactString;
use std::rc::Rc;

pub type CommandCallback = Rc<v8::Global<v8::Function>>;

#[derive_where::derive_where(Debug)]
#[derive(Clone, rsvim_macro::ToV8, rsvim_macro::RcPtr)]
pub struct CommandDefinition {
  pub name: CompactString,
  #[derive_where(skip)]
  pub callback: CommandCallback,
  pub attributes: CommandAttributes,
  pub options: CommandOptions,
}

impl FromV8CallbackArgs for CommandDefinition {
  fn from_v8_callback_args<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    args: v8::FunctionCallbackArguments<'s>,
  ) -> Self {
    debug_assert!(args.length() == 4);
    debug_assert!(is_v8_str!(args.get(0)));
    let name = args.get(0).to_rust_string_lossy(scope);
    debug_assert!(is_v8_func!(args.get(1)));
    let callback = v8::Local::<v8::Function>::try_from(args.get(1)).unwrap();
    let callback = Rc::new(v8::Global::new(scope, callback));
    debug_assert!(args.get(2).is_object());
    let attributes = CommandAttributes::from_v8(scope, args.get(2));
    debug_assert!(args.get(3).is_object());
    let options = CommandOptions::from_v8(scope, args.get(3));

    Self {
      name: name.to_compact_string(),
      callback,
      attributes,
      options,
    }
  }
}
