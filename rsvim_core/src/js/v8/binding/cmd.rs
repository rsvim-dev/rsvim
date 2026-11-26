//! APIs for `cmd` module.

use crate::is_v8_str;
use crate::js::JsKernel;
use crate::js::binding;
use crate::js::command::def::CommandDefinition;
use crate::js::converter::*;
use crate::prelude::*;
use compact_str::CompactString;
use compact_str::ToCompactString;

/// Forwards to the engine-neutral implementation.
///
/// Kept as a thin shim rather than rewriting the four call sites: `cmd.rs` and
/// `err.rs` both reach it, and the body has nothing v8-specific in it.
pub fn send_cmdline_message(state: &JsKernel, payload: String) {
  crate::js::cmdline::send_cmdline_message(&state.editor, payload);
}

/// `cmd.echo` API.
pub fn echo(
  scope: &mut v8::PinScope,
  args: v8::FunctionCallbackArguments,
  mut _rv: v8::ReturnValue,
) {
  debug_assert!(args.length() == 1);
  debug_assert!(!args.get(0).is_null_or_undefined());
  let message = args.get(0).to_rust_string_lossy(scope);
  trace!("cmd.echo:{:?}", message);

  let state_rc = crate::js::get_state(scope);
  let state = state_rc.borrow();
  send_cmdline_message(&state, message);
}

/// `cmd.create` API.
pub fn create<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  let def = CommandDefinition::from_v8_callback_args(scope, args);
  trace!("cmd.create:{:?}", def);

  let state_rc = crate::js::get_state(scope);
  let state = state_rc.borrow_mut();
  let mut command_manager = lock!(state.editor.command_manager);

  let result = command_manager
    .insert(def.name.to_compact_string(), CommandDefinition::to_rc(def));

  match result {
    Ok(Some(removed)) => rv.set(removed.to_v8(scope)),
    Ok(None) => rv.set_undefined(),
    Err(e) => {
      binding::throw_exception(scope, &e);
    }
  }
}

/// `cmd.list` API.
pub fn list(
  scope: &mut v8::PinScope,
  _args: v8::FunctionCallbackArguments,
  mut rv: v8::ReturnValue,
) {
  debug_assert!(_args.length() == 0);
  trace!("cmd.list");

  let state_rc = crate::js::get_state(scope);
  let state = state_rc.borrow_mut();
  let command_manager = lock!(state.editor.command_manager);

  let commands = command_manager
    .keys()
    .collect::<Vec<&CompactString>>()
    .to_v8(scope);

  rv.set(commands);
}

/// `cmd.get` API.
pub fn get<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  debug_assert!(args.length() == 1);
  debug_assert!(is_v8_str!(args.get(0)));
  let name = args.get(0).to_rust_string_lossy(scope);
  trace!("cmd.get:{:?}", name);

  let state_rc = crate::js::get_state(scope);
  let state = state_rc.borrow_mut();
  let command_manager = lock!(state.editor.command_manager);
  match command_manager.get(&name) {
    Some(def) => rv.set(def.to_v8(scope)),
    None => rv.set_undefined(),
  }
}

/// `cmd.remove` API.
pub fn remove<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  debug_assert!(args.length() == 1);
  debug_assert!(is_v8_str!(args.get(0)));
  let name = args.get(0).to_rust_string_lossy(scope);
  trace!("cmd.remove:{:?}", name);

  let state_rc = crate::js::get_state(scope);
  let state = state_rc.borrow_mut();
  let mut command_manager = lock!(state.editor.command_manager);
  match command_manager.remove(&name) {
    Some(removed) => rv.set(removed.to_v8(scope)),
    None => rv.set_undefined(),
  }
}
