//! Ex command definition.

use crate::js::command::attr::*;
use crate::js::command::opt::*;
use compact_str::CompactString;

#[cfg(feature = "js-v8")]
use crate::is_v8_func;
#[cfg(feature = "js-v8")]
use crate::is_v8_str;
#[cfg(feature = "js-v8")]
use crate::js::converter::*;
#[cfg(feature = "js-v8")]
use compact_str::ToCompactString;
#[cfg(feature = "js-v8")]
use std::rc::Rc;

/// The javascript function a user registered with `cmd.create(name, cb)`.
///
/// Each engine spells "a callable that outlives the current scope" differently:
/// v8 needs a [`v8::Global`] because a [`v8::Local`] dies with its scope, and
/// rquickjs needs a [`rquickjs::Persistent`] for the same reason. The rest of
/// the command machinery only ever stores the handle and hands it back, so an
/// enum is enough — and it has to be an enum rather than a `#[cfg]` alias
/// because both engines can be compiled at once.
///
/// NOTE: a `Persistent` that outlives its runtime makes QuickJS abort the
/// process on drop. The command manager lives in [`crate::evloop::Editor`] and
/// the runtime in the same struct, so the field order there decides which one
/// drops first.
#[derive(Clone)]
pub enum CommandCallback {
  #[cfg(feature = "js-v8")]
  V8(Rc<v8::Global<v8::Function>>),

  #[cfg(feature = "js-quickjs")]
  Rqjs(rquickjs::Persistent<rquickjs::Function<'static>>),
}

impl std::fmt::Debug for CommandCallback {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      #[cfg(feature = "js-v8")]
      Self::V8(_) => f.write_str("CommandCallback::V8"),
      #[cfg(feature = "js-quickjs")]
      Self::Rqjs(_) => f.write_str("CommandCallback::Rqjs"),
    }
  }
}

#[cfg(feature = "js-v8")]
impl ToV8 for CommandCallback {
  fn to_v8<'s>(
    &self,
    scope: &mut v8::PinScope<'s, '_>,
  ) -> v8::Local<'s, v8::Value> {
    match self {
      Self::V8(cb) => cb.to_v8(scope),
      // Unreachable while only the v8 backend is selected: a handle can only
      // have been created by whichever engine was built.
      #[cfg(feature = "js-quickjs")]
      Self::Rqjs(_) => v8::undefined(scope).into(),
    }
  }
}

#[cfg(feature = "js-v8")]
impl CommandCallback {
  /// The v8 handle, for the call path that has to turn it into a
  /// `v8::Local` inside a scope.
  ///
  /// A build that only contains the v8 backend has exactly one variant, so the
  /// quickjs arm below is compiled out and the `match` is total either way.
  pub fn as_v8(&self) -> &Rc<v8::Global<v8::Function>> {
    match self {
      Self::V8(cb) => cb,
      #[cfg(feature = "js-quickjs")]
      Self::Rqjs(_) => {
        unreachable!("a v8 command callback can only come from the v8 engine")
      }
    }
  }
}

#[derive_where::derive_where(Debug)]
#[derive(Clone, rsvim_macro::RcPtr)]
#[cfg_attr(feature = "js-v8", derive(rsvim_macro::ToV8))]
pub struct CommandDefinition {
  pub name: CompactString,
  #[derive_where(skip)]
  pub callback: CommandCallback,
  pub attributes: CommandAttributes,
  pub options: CommandOptions,
}

#[cfg(feature = "js-v8")]
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
    let callback =
      CommandCallback::V8(Rc::new(v8::Global::new(scope, callback)));
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
