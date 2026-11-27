//! APIs for the `process` module.
//!
//! The synchronous half only: `exit`, plus `spawn` (which is synchronous on the
//! Rust side — it only launches the child and hands back resource ids).
//! `wait`/`text` are promise-returning and land with the async bindings.

use crate::chan;
use crate::chan::MasterMessage;
use crate::js::binding::set_function_to;
use crate::js::runtime::kernel;
use crate::prelude::*;
use compact_str::CompactString;
use compact_str::ToCompactString;
use rquickjs::Ctx;
use rquickjs::Exception;
use rquickjs::Object;
use rquickjs::Result as JsResult;
use std::str::FromStr;

mod command;

use crate::js::binding::process::command::ProcessCommandOptions;
use crate::js::binding::process::command::ProcessCommandOptionsBuilder;
use crate::js::binding::process::command::Stdio;
use crate::js::binding::process::command::spawn_child_process;

/// Mirrors the v8 backend's `ProcessCommandOptions::from_v8`, but reads the
/// object directly: `rquickjs` has no `#[derive(FromJs)]`, and the field set is
/// small enough that an explicit reader stays readable.
///
/// Every field is optional in javascript — `process.spawn("ls")` must work — so
/// a missing property keeps the builder default instead of raising.
fn options_from_object(object: &Object<'_>) -> JsResult<ProcessCommandOptions> {
  let mut builder = ProcessCommandOptionsBuilder::default();

  if let Some(args) = object.get::<&str, Option<Vec<String>>>("args")? {
    builder.args(args.into_iter().map(CompactString::from).collect());
  }
  if let Some(cwd) = object.get::<&str, Option<String>>("cwd")? {
    builder.cwd(Some(CompactString::from(cwd)));
  }
  if let Some(clear_env) = object.get::<&str, Option<bool>>("clearEnv")? {
    builder.clear_env(clear_env);
  }
  if let Some(detached) = object.get::<&str, Option<bool>>("detached")? {
    builder.detached(detached);
  }
  if let Some(env) = object.get::<&str, Option<Object<'_>>>("env")? {
    let mut map = FoldMap::default();
    for entry in env.props::<String, String>() {
      let (key, value) = entry?;
      map.insert(CompactString::from(key), CompactString::from(value));
    }
    builder.env(map);
  }
  if let Some(stdin) = object.get::<&str, Option<String>>("stdin")? {
    builder.stdin(parse_stdio(&stdin));
  }
  if let Some(stdout) = object.get::<&str, Option<String>>("stdout")? {
    builder.stdout(parse_stdio(&stdout));
  }
  if let Some(stderr) = object.get::<&str, Option<String>>("stderr")? {
    builder.stderr(parse_stdio(&stderr));
  }

  Ok(builder.build().unwrap())
}

/// Unknown stdio names fall back to the builder default rather than panicking
/// the way the v8 backend's `Stdio::from_str(..).unwrap()` does. A typo in a
/// config file should not take the editor down.
fn parse_stdio(value: &str) -> Stdio {
  Stdio::from_str(value).unwrap_or(Stdio::Null)
}

/// `process.spawn(execPath, options)`.
pub fn spawn_child<'js>(
  ctx: Ctx<'js>,
  exec_path: String,
  options: Option<Object<'js>>,
) -> JsResult<Object<'js>> {
  let options = match options {
    Some(object) => options_from_object(&object)?,
    None => ProcessCommandOptionsBuilder::default().build().unwrap(),
  };
  trace!(
    "spawn_child exec_path: {:?}, options: {:?}",
    exec_path, options
  );

  let kernel = kernel(&ctx)?;
  let resource_table = kernel.borrow().editor.resource_table.clone();

  let (child_rid, stdin_rid, stdout_rid, stderr_rid) =
    spawn_child_process(resource_table, &exec_path, &options)
      .map_err(|e| Exception::throw_message(&ctx, &e.to_string()))?;

  let result = Object::new(ctx.clone())?;
  result.set("execPath", exec_path.to_compact_string().to_string())?;
  result.set("options", options_to_object(&ctx, &options)?)?;
  result.set("rid", i32::from(child_rid))?;
  if let Some(rid) = stdin_rid {
    result.set("stdinRid", i32::from(rid))?;
  }
  if let Some(rid) = stdout_rid {
    result.set("stdoutRid", i32::from(rid))?;
  }
  if let Some(rid) = stderr_rid {
    result.set("stderrRid", i32::from(rid))?;
  }
  Ok(result)
}

/// Serialises `ProcessCommandOptions` back for the javascript side.
///
/// The v8 backend derives `ToV8`, which lowercases the first letter of every
/// field name; the same names are used here so `process.min.js` sees an
/// identical shape.
fn options_to_object<'js>(
  ctx: &Ctx<'js>,
  options: &ProcessCommandOptions,
) -> JsResult<Object<'js>> {
  let object = Object::new(ctx.clone())?;
  object.set(
    "args",
    options
      .args
      .iter()
      .map(|a| a.to_string())
      .collect::<Vec<String>>(),
  )?;
  object.set("cwd", options.cwd.as_ref().map(|c| c.to_string()))?;
  object.set("clearEnv", options.clear_env)?;
  object.set("detached", options.detached)?;
  let env = Object::new(ctx.clone())?;
  for (key, value) in options.env.iter() {
    env.set(key.as_str(), value.as_str())?;
  }
  object.set("env", env)?;
  object.set("stdin", options.stdin.to_string())?;
  object.set("stdout", options.stdout.to_string())?;
  object.set("stderr", options.stderr.to_string())?;
  Ok(object)
}

/// `process.exit(code)`.
pub fn exit<'js>(ctx: Ctx<'js>, exit_code: i32) -> JsResult<()> {
  trace!("exit: {:?}", exit_code);
  let kernel = kernel(&ctx)?;
  let master_tx = kernel.borrow().master_tx.clone();
  chan::send_to_master(
    master_tx,
    MasterMessage::ExitReq(chan::ExitReq { exit_code }),
  );
  Ok(())
}

/// Installs the `process_*` built-ins on `internal`.
pub fn install<'js>(ctx: &Ctx<'js>, internal: &Object<'js>) -> JsResult<()> {
  set_function_to(ctx, internal, "process_exit", exit)?;
  set_function_to(ctx, internal, "process_spawn_child", spawn_child)?;
  Ok(())
}
