//! APIs for `process` module.

pub mod child_process;

use crate::chan;
use crate::chan::MasterMessage;
use crate::is_v8_int;
use crate::is_v8_str;
use crate::js;
use crate::js::binding;
use crate::js::binding::process::child_process::ReadTextFromChildProcessStdioFuture;
use crate::js::binding::process::child_process::WaitChildFuture;
use crate::js::process_command::spawn_child_process;
use crate::js::converter::*;
use crate::js::pending;
use crate::js::resource::ResourceId;
use crate::prelude::*;
use compact_str::ToCompactString;
use crate::js::process_command::ProcessCommandOptions;

/// The `spawn()` method in `process.Command`.
pub fn spawn_child<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  debug_assert!(args.length() == 2);
  debug_assert!(is_v8_str!(args.get(0)));
  let exec_path = args.get(0).to_rust_string_lossy(scope);
  let options = ProcessCommandOptions::from_v8(scope, args.get(1));
  trace!(
    "spawn_child exec_path: {:?}, options: {:?}",
    exec_path, options
  );

  let state_rc = crate::js::get_state(scope);
  let resource_table = state_rc.borrow().editor.resource_table.clone();

  match spawn_child_process(resource_table, &exec_path, &options) {
    Ok((child_rid, stdin_rid, stdout_rid, stderr_rid)) => {
      let result = v8::Object::new(scope);
      let exec_path = exec_path.to_compact_string().to_v8(scope);
      binding::set_property_to(scope, result, "execPath", exec_path);
      let options = options.to_v8(scope);
      binding::set_property_to(scope, result, "options", options);
      let child_rid = Into::<i32>::into(child_rid).to_v8(scope);
      binding::set_property_to(scope, result, "rid", child_rid);
      if let Some(stdin_rid) = stdin_rid {
        let stdin_rid = Into::<i32>::into(stdin_rid).to_v8(scope);
        binding::set_property_to(scope, result, "stdinRid", stdin_rid);
      }
      if let Some(stdout_rid) = stdout_rid {
        let stdout_rid = Into::<i32>::into(stdout_rid).to_v8(scope);
        binding::set_property_to(scope, result, "stdoutRid", stdout_rid);
      }
      if let Some(stderr_rid) = stderr_rid {
        let stderr_rid = Into::<i32>::into(stderr_rid).to_v8(scope);
        binding::set_property_to(scope, result, "stderrRid", stderr_rid);
      }
      rv.set(result.into());
    }
    Err(e) => {
      binding::throw_exception(scope, &e);
    }
  }
}

/// The `text()` method in `process.ChildProcessReadableStream` class.
pub fn read_text_from_child<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  debug_assert!(args.length() == 1);
  debug_assert!(is_v8_int!(args.get(0)));
  let rid = i32::from_v8(scope, args.get(0));
  let rid = ResourceId::from(rid);
  trace!("read_text_from_child rid: {:?}", rid);

  let promise_resolver = v8::PromiseResolver::new(scope).unwrap();
  let promise = promise_resolver.get_promise(scope);

  let state_rc = crate::js::get_state(scope);
  let read_cb = {
    let promise = v8::Global::new(scope, promise_resolver);
    let state_rc = state_rc.clone();
    move |maybe_result: Option<TheResult<Vec<u8>>>| {
      let fut = ReadTextFromChildProcessStdioFuture {
        promise: promise.clone(),
        maybe_result,
      };
      let mut state = state_rc.borrow_mut();
      state.pending_futures.push(Box::new(fut));
    }
  };

  let mut state = state_rc.borrow_mut();
  let task_id = js::TaskId::next();
  pending::create_read_text_from_child_process_stdio(
    &mut state,
    task_id,
    rid,
    Box::new(read_cb),
  );

  rv.set(promise.into());
}

/// The `wait()` method in `process.ChildProcess`.
pub fn wait_child<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  debug_assert!(args.length() == 1);
  debug_assert!(is_v8_int!(args.get(0)));
  let rid = i32::from_v8(scope, args.get(0));
  let rid = ResourceId::from(rid);
  trace!("wait_child rid: {:?}", rid);

  let promise_resolver = v8::PromiseResolver::new(scope).unwrap();
  let promise = promise_resolver.get_promise(scope);

  let state_rc = crate::js::get_state(scope);
  let wait_cb = {
    let promise = v8::Global::new(scope, promise_resolver);
    let state_rc = state_rc.clone();
    move |maybe_result: Option<TheResult<Vec<u8>>>| {
      let fut = WaitChildFuture {
        promise: promise.clone(),
        maybe_result,
      };
      let mut state = state_rc.borrow_mut();
      state.pending_futures.push(Box::new(fut));
    }
  };

  let mut state = state_rc.borrow_mut();
  let task_id = js::TaskId::next();
  pending::create_wait_child_process(
    &mut state,
    task_id,
    rid,
    Box::new(wait_cb),
  );

  rv.set(promise.into());
}

/// Exit editor.
pub fn exit<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut _rv: v8::ReturnValue,
) {
  debug_assert!(args.length() == 1);
  debug_assert!(is_v8_int!(args.get(0)));
  let exit_code = i32::from_v8(scope, args.get(0));
  trace!("exit: {:?}", exit_code);

  let state_rc = crate::js::get_state(scope);
  let state = state_rc.borrow();

  chan::send_to_master(
    state.master_tx.clone(),
    MasterMessage::ExitReq(chan::ExitReq { exit_code }),
  );
}
