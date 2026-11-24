//! APIs for `opt` module.

use crate::buf::opt::FileEncodingOption;
use crate::buf::opt::FileFormatOption;
use crate::is_v8_bool;
use crate::is_v8_int;
use crate::is_v8_str;
use crate::js::converter::*;
use crate::prelude::*;
use compact_str::ToCompactString;

/// Get the _wrap_ option.
/// See: <https://vimhelp.org/options.txt.html#%27wrap%27>
/// Also known as _line-wrap_, see: <https://en.wikipedia.org/wiki/Line_wrap_and_word_wrap>.
pub fn get_wrap(
  scope: &mut v8::PinScope,
  _args: v8::FunctionCallbackArguments,
  mut rv: v8::ReturnValue,
) {
  let state_rc = crate::js::get_state(scope);
  let tree = state_rc.borrow().editor.tree.clone();
  let tree = lock!(tree);
  let value = tree.global_local_options().wrap();
  trace!("get_wrap: {:?}", value);
  rv.set_bool(value);
}

/// Set the _wrap_ option.
pub fn set_wrap<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut _rv: v8::ReturnValue,
) {
  debug_assert!(args.length() == 1);
  debug_assert!(is_v8_bool!(args.get(0)));
  let value = bool::from_v8(scope, args.get(0));
  trace!("set_wrap: {:?}", value);
  let state_rc = crate::js::get_state(scope);
  let tree = state_rc.borrow().editor.tree.clone();
  let mut tree = lock!(tree);
  tree.global_local_options_mut().set_wrap(value);
}

/// Get the _line-break_ option.
/// See: <https://vimhelp.org/options.txt.html#%27linebreak%27>
/// Also known as _word-wrap_, see: <https://en.wikipedia.org/wiki/Line_wrap_and_word_wrap>.
pub fn get_line_break(
  scope: &mut v8::PinScope,
  _args: v8::FunctionCallbackArguments,
  mut rv: v8::ReturnValue,
) {
  let state_rc = crate::js::get_state(scope);
  let tree = state_rc.borrow().editor.tree.clone();
  let tree = lock!(tree);
  let value = tree.global_local_options().line_break();
  trace!("get_line_break: {:?}", value);
  rv.set_bool(value);
}

/// Set the _line-break_ option.
pub fn set_line_break<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut _rv: v8::ReturnValue,
) {
  debug_assert!(args.length() == 1);
  debug_assert!(is_v8_bool!(args.get(0)));
  let value = bool::from_v8(scope, args.get(0));
  trace!("set_line_break: {:?}", value);
  let state_rc = crate::js::get_state(scope);
  let tree = state_rc.borrow().editor.tree.clone();
  let mut tree = lock!(tree);
  tree.global_local_options_mut().set_line_break(value);
}

/// Get the _tap-stop_ option.
/// See: <https://vimhelp.org/options.txt.html#%27tabstop%27>
pub fn get_tab_stop(
  scope: &mut v8::PinScope,
  _args: v8::FunctionCallbackArguments,
  mut rv: v8::ReturnValue,
) {
  let state_rc = crate::js::get_state(scope);
  let buffer_manager = state_rc.borrow().editor.buffer_manager.clone();
  let buffer_manager = lock!(buffer_manager);
  let value = buffer_manager.global_local_options().tab_stop();
  trace!("get_tab_stop: {:?}", value);
  rv.set_int32(value as i32);
}

/// Set the _tab-stop_ option.
pub fn set_tab_stop<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  _: v8::ReturnValue,
) {
  debug_assert!(args.length() == 1);
  debug_assert!(is_v8_int!(args.get(0)));
  let value = u32::from_v8(scope, args.get(0));
  trace!("set_tab_stop: {:?}", value);
  let state_rc = crate::js::get_state(scope);
  let buffer_manager = state_rc.borrow().editor.buffer_manager.clone();
  let mut buffer_manager = lock!(buffer_manager);

  debug_assert!(value <= u8::MAX as u32);
  buffer_manager
    .global_local_options_mut()
    .set_tab_stop(value as u8);
}

/// Get the _expand-tab_ option.
/// See: <https://vimhelp.org/options.txt.html#%27expandtab%27>
pub fn get_expand_tab(
  scope: &mut v8::PinScope,
  _args: v8::FunctionCallbackArguments,
  mut rv: v8::ReturnValue,
) {
  let state_rc = crate::js::get_state(scope);
  let buffer_manager = state_rc.borrow().editor.buffer_manager.clone();
  let buffer_manager = lock!(buffer_manager);
  let value = buffer_manager.global_local_options().expand_tab();
  trace!("get_expand_tab: {:?}", value);
  rv.set_bool(value);
}

/// Set the _expand-tab_ option.
pub fn set_expand_tab<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  _: v8::ReturnValue,
) {
  debug_assert!(args.length() == 1);
  debug_assert!(is_v8_bool!(args.get(0)));
  let value = bool::from_v8(scope, args.get(0));
  trace!("set_expand_tab: {:?}", value);
  let state_rc = crate::js::get_state(scope);
  let buffer_manager = state_rc.borrow().editor.buffer_manager.clone();
  let mut buffer_manager = lock!(buffer_manager);

  buffer_manager
    .global_local_options_mut()
    .set_expand_tab(value);
}

/// Get the _shift-width_ option.
/// See: <https://vimhelp.org/options.txt.html#%27shiftwidth%27>
pub fn get_shift_width(
  scope: &mut v8::PinScope,
  _args: v8::FunctionCallbackArguments,
  mut rv: v8::ReturnValue,
) {
  let state_rc = crate::js::get_state(scope);
  let buffer_manager = state_rc.borrow().editor.buffer_manager.clone();
  let buffer_manager = lock!(buffer_manager);
  let value = buffer_manager.global_local_options().shift_width();
  trace!("get_shift_width: {:?}", value);
  rv.set_int32(value as i32);
}

/// Set the _shift-width_ option.
pub fn set_shift_width<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  _: v8::ReturnValue,
) {
  debug_assert!(args.length() == 1);
  debug_assert!(is_v8_int!(args.get(0)));
  let value = u32::from_v8(scope, args.get(0));
  trace!("set_shift_width: {:?}", value);
  let state_rc = crate::js::get_state(scope);
  let buffer_manager = state_rc.borrow().editor.buffer_manager.clone();
  let mut buffer_manager = lock!(buffer_manager);

  debug_assert!(value <= u8::MAX as u32);
  buffer_manager
    .global_local_options_mut()
    .set_shift_width(value as u8);
}

/// Get the _file-encoding_ option.
/// See: <https://vimhelp.org/options.txt.html#%27fileencoding%27>
pub fn get_file_encoding(
  scope: &mut v8::PinScope,
  _args: v8::FunctionCallbackArguments,
  mut rv: v8::ReturnValue,
) {
  let state_rc = crate::js::get_state(scope);
  let buffer_manager = state_rc.borrow().editor.buffer_manager.clone();
  let buffer_manager = lock!(buffer_manager);
  let value = buffer_manager.global_local_options().file_encoding();
  trace!("get_file_encoding: {:?}", value);
  let value = value.to_compact_string().to_v8(scope);
  rv.set(value);
}

/// Set the _file-encoding_ option.
pub fn set_file_encoding<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  _: v8::ReturnValue,
) {
  debug_assert!(args.length() == 1);
  debug_assert!(is_v8_str!(args.get(0)));
  let value = args.get(0).to_rust_string_lossy(scope).to_lowercase();
  trace!("set_file_encoding: {:?}", value);
  let state_rc = crate::js::get_state(scope);
  let buffer_manager = state_rc.borrow().editor.buffer_manager.clone();
  let mut buffer_manager = lock!(buffer_manager);

  let value = FileEncodingOption::try_from(value.as_str()).unwrap();
  buffer_manager
    .global_local_options_mut()
    .set_file_encoding(value);
}

/// Get the _file-format_ option.
/// See: <https://vimhelp.org/options.txt.html#%27fileformat%27>
pub fn get_file_format(
  scope: &mut v8::PinScope,
  _args: v8::FunctionCallbackArguments,
  mut rv: v8::ReturnValue,
) {
  let state_rc = crate::js::get_state(scope);
  let buffer_manager = state_rc.borrow().editor.buffer_manager.clone();
  let buffer_manager = lock!(buffer_manager);
  let value = buffer_manager.global_local_options().file_format();
  trace!("get_file_format: {:?}", value);
  let value = value.to_compact_string().to_v8(scope);
  rv.set(value);
}

/// Set the _file-format_ option.
pub fn set_file_format<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  _: v8::ReturnValue,
) {
  debug_assert!(args.length() == 1);
  debug_assert!(is_v8_str!(args.get(0)));
  let value = args.get(0).to_rust_string_lossy(scope).to_lowercase();
  trace!("set_file_format: {:?}", value);
  let state_rc = crate::js::get_state(scope);
  let buffer_manager = state_rc.borrow().editor.buffer_manager.clone();
  let mut buffer_manager = lock!(buffer_manager);

  let value = FileFormatOption::try_from(value.as_str()).unwrap();
  buffer_manager
    .global_local_options_mut()
    .set_file_format(value);
}

/// Get the _fix-end-of-line_ option.
/// See: <https://vimhelp.org/options.txt.html#%27fixendofline%27>
pub fn get_fix_end_of_line(
  scope: &mut v8::PinScope,
  _args: v8::FunctionCallbackArguments,
  mut rv: v8::ReturnValue,
) {
  let state_rc = crate::js::get_state(scope);
  let buffer_manager = state_rc.borrow().editor.buffer_manager.clone();
  let buffer_manager = lock!(buffer_manager);
  let value = buffer_manager.global_local_options().fix_end_of_line();
  trace!("get_fix_end_of_line: {:?}", value);
  rv.set_bool(value);
}

/// Set the _fix-end-of-line_ option.
pub fn set_fix_end_of_line<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  _: v8::ReturnValue,
) {
  debug_assert!(args.length() == 1);
  debug_assert!(is_v8_bool!(args.get(0)));
  let value = bool::from_v8(scope, args.get(0));
  trace!("set_fix_end_of_line: {:?}", value);
  let state_rc = crate::js::get_state(scope);
  let buffer_manager = state_rc.borrow().editor.buffer_manager.clone();
  let mut buffer_manager = lock!(buffer_manager);

  buffer_manager
    .global_local_options_mut()
    .set_fix_end_of_line(value);
}

/// Get the _syntax-parser-lib-path_ option.
pub fn get_syntax_parser_lib_path(
  scope: &mut v8::PinScope,
  _args: v8::FunctionCallbackArguments,
  mut rv: v8::ReturnValue,
) {
  let state_rc = crate::js::get_state(scope);
  let syntax_manager = state_rc.borrow().editor.syntax_manager.clone();
  let syntax_manager = lock!(syntax_manager);
  let value = syntax_manager
    .loader()
    .treesitter_parser_lib_path()
    .to_string_lossy()
    .to_string();
  trace!("get_syntax_parser_lib_path: {:?}", value);
  let value = value.to_v8(scope);
  rv.set(value);
}

/// Set the _syntax-parser-lib-path_ option.
pub fn set_syntax_parser_lib_path<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  _: v8::ReturnValue,
) {
  debug_assert!(args.length() == 1);
  debug_assert!(is_v8_str!(args.get(0)));
  let value = args.get(0).to_rust_string_lossy(scope);
  trace!("set_syntax_parser_lib_path: {:?}", value);
  let state_rc = crate::js::get_state(scope);
  let syntax_manager = state_rc.borrow().editor.syntax_manager.clone();
  let syntax_manager = lock!(syntax_manager);
  syntax_manager
    .loader()
    .set_treesitter_parser_lib_path(Path::new(&value).to_path_buf());
}
