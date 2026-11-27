//! APIs for the `process` module — the promise-returning half.
//!
//! `ChildProcess.wait()` and `ChildProcessReadableStream.text()` block on the
//! operating system, so they run on tokio's blocking pool, as
//! [`OpKind::Blocked`] operations: `Rsvim::close_trackers` waits for a `wait()`
//! that was already promised to javascript — exiting in the middle of one would
//! leave the promise dangling with no diagnostic.

use crate::js::binding::set_function_to;
use crate::js::ops::OpKind;
use crate::js::ops::blocking;
use crate::js::ops::spawn_op;
use crate::js::ops::spawn_simple_op;
use crate::js::resource::Resource;
use crate::js::resource::ResourceId;
use crate::js::runtime::kernel;
use crate::prelude::*;
use rquickjs::Ctx;
use rquickjs::Object;
use rquickjs::Promise;
use rquickjs::Result as JsResult;

/// `childProcess.wait()` -> `Promise<{success, code?, signal?}>`.
///
/// The object mirrors the v8 backend's: `success` is always present, `code` and
/// `signal` only when the platform reported them.
pub fn wait_child<'js>(ctx: Ctx<'js>, rid: i32) -> JsResult<Promise<'js>> {
  let kernel = kernel(&ctx)?;
  let resource_table = kernel.borrow().editor.resource_table.clone();
  let rid = ResourceId::from(rid);
  trace!("process.wait:{:?}", rid);

  spawn_op(
    &ctx,
    "process.wait",
    OpKind::Blocked,
    None,
    blocking(
      "process.wait",
      move || -> TheResult<(bool, Option<i32>, Option<i32>)> {
        let child = lock!(resource_table).get(&rid).cloned();
        let Some(child) = child else {
          return Err(TheErr::ChildProcessNotFound(rid));
        };
        let Resource::ChildProcess(resource) = child else {
          unreachable!()
        };
        let resource = resource.data();
        let mut resource = lock!(resource);
        let exit_status = resource
          .wait()
          .map_err(|e| TheErr::WaitChildProcessFailed(rid, e))?;
        let success = exit_status.success();
        let exit_code = exit_status.code();
        let signal: Option<i32> = {
          #[cfg(target_family = "unix")]
          {
            use std::os::unix::process::ExitStatusExt;
            exit_status.signal()
          }
          #[cfg(not(target_family = "unix"))]
          {
            None
          }
        };
        Ok((success, exit_code, signal))
      },
    ),
    |ctx,
     _aux,
     (success, exit_code, signal): (bool, Option<i32>, Option<i32>)| {
      let object = Object::new(ctx.clone())?;
      object.set("success", success)?;
      if let Some(exit_code) = exit_code {
        object.set("code", exit_code)?;
      }
      if let Some(signal) = signal {
        object.set("signal", signal)?;
      }
      Ok(object.into_value())
    },
  )
}

/// `childProcessReadableStream.text()` -> `Promise<string>`.
pub fn read_text_from_child<'js>(
  ctx: Ctx<'js>,
  rid: i32,
) -> JsResult<Promise<'js>> {
  let kernel = kernel(&ctx)?;
  let resource_table = kernel.borrow().editor.resource_table.clone();
  let rid = ResourceId::from(rid);
  trace!("process.text:{:?}", rid);

  spawn_simple_op(
    &ctx,
    "process.text",
    OpKind::Blocked,
    blocking("process.text", move || -> TheResult<String> {
      use std::io::Read;

      let child = lock!(resource_table).get(&rid).cloned();
      let Some(child) = child else {
        return Err(TheErr::ChildProcessNotFound(rid));
      };
      let mut payload = String::new();
      match child {
        Resource::ChildProcessStdout(resource) => {
          let resource = resource.data();
          let mut resource = lock!(resource);
          resource
            .read_to_string(&mut payload)
            .map_err(|e| TheErr::ReadChildProcessStdioFailed(rid, e))?;
        }
        Resource::ChildProcessStderr(resource) => {
          let resource = resource.data();
          let mut resource = lock!(resource);
          resource
            .read_to_string(&mut payload)
            .map_err(|e| TheErr::ReadChildProcessStdioFailed(rid, e))?;
        }
        _ => unreachable!(),
      }
      Ok(payload)
    }),
  )
}

/// Installs the promise-returning `process_*` built-ins on `internal`.
pub fn install<'js>(ctx: &Ctx<'js>, internal: &Object<'js>) -> JsResult<()> {
  set_function_to(ctx, internal, "process_wait_child", wait_child)?;
  set_function_to(
    ctx,
    internal,
    "process_read_text_from_child",
    read_text_from_child,
  )?;
  Ok(())
}
