//! APIs for the `cmd` module.

use crate::js::cmdline::send_cmdline_message;
use crate::js::command::attr::CommandAttributes;
use crate::js::command::attr::CommandNargs;
use crate::js::command::def::CommandCallback;
use crate::js::command::def::CommandDefinition;
use crate::js::command::def::CommandDefinitionRc;
use crate::js::command::opt::CommandOptions;
use crate::js::rqjs::binding::set_function_to;
use crate::js::rqjs::get_state;
use crate::prelude::*;
use compact_str::ToCompactString;
use rquickjs::Ctx;
use rquickjs::Exception;
use rquickjs::Function;
use rquickjs::Object;
use rquickjs::Persistent;
use rquickjs::Result as JsResult;
use rquickjs::Value;
use rquickjs::function::Opt;
use std::str::FromStr;

/// `cmd.echo(message)`.
///
/// The javascript wrapper in `rsvim_core/js/cmd.min.js` has already checked that
/// the argument is not nil, so this takes an owned `String` and lets `rquickjs`
/// coerce whatever it was given.
pub fn echo<'js>(ctx: Ctx<'js>, message: String) -> JsResult<()> {
  let kernel = get_state(&ctx).ok_or_else(|| {
    Exception::throw_message(&ctx, "js kernel is not initialised")
  })?;
  send_cmdline_message(&kernel.borrow().editor, message);
  Ok(())
}

/// Reads `nargs` out of the attributes object.
fn nargs_from<'js>(
  ctx: &Ctx<'js>,
  value: Value<'js>,
) -> JsResult<CommandNargs> {
  let Some(text) = value.as_string() else {
    // The javascript side defaults the whole object, so a missing `nargs` never
    // reaches here; a wrong type is a genuine programming error.
    return Err(Exception::throw_type(ctx, "nargs must be a string"));
  };
  let text = text.to_string()?;
  CommandNargs::from_str(&text)
    .map_err(|_| Exception::throw_range(ctx, "unknown nargs"))
}

/// Reads `{bang, nargs}`.
///
/// `cmd.min.js` fills in the defaults before the call, so both fields are
/// present; the fallbacks below only guard a caller that bypasses the wrapper.
fn attributes_from<'js>(
  ctx: &Ctx<'js>,
  value: Value<'js>,
) -> JsResult<CommandAttributes> {
  let Some(obj) = value.as_object() else {
    return Ok(CommandAttributes {
      bang: false,
      nargs: CommandNargs::Zero,
    });
  };
  let bang: bool = obj.get("bang").unwrap_or(false);
  let nargs = match obj.get::<_, Value>("nargs") {
    Ok(v) if !v.is_undefined() => nargs_from(ctx, v)?,
    _ => CommandNargs::Zero,
  };
  Ok(CommandAttributes { bang, nargs })
}

/// Reads `{force, alias}`.
fn options_from<'js>(
  ctx: &Ctx<'js>,
  value: Value<'js>,
) -> JsResult<CommandOptions> {
  let Some(obj) = value.as_object() else {
    return Ok(CommandOptions {
      force: true,
      alias: None,
    });
  };
  let force: bool = obj.get("force").unwrap_or(true);
  let alias = match obj.get::<_, Value>("alias") {
    Ok(v) if !v.is_undefined() => match v.as_string() {
      Some(s) => Some(s.to_string()?.to_compact_string()),
      None => None,
    },
    _ => None,
  };
  let _ = ctx;
  Ok(CommandOptions { force, alias })
}

/// `cmd.create(name, callback, attributes, options)`.
///
/// Returns the definition that was replaced, or `undefined` — the same contract
/// as v8's `cmd_create`, and the reason `cmd.min.js` can print
/// `Previous command:${prev}`.
pub fn create<'js>(
  ctx: Ctx<'js>,
  name: String,
  callback: Function<'js>,
  attributes: Opt<Value<'js>>,
  options: Opt<Value<'js>>,
) -> JsResult<Value<'js>> {
  // `Opt` rather than a plain parameter: a caller may legitimately stop after
  // the callback, and rquickjs rejects a missing non-optional argument.
  let attributes = match attributes.0 {
    Some(v) => attributes_from(&ctx, v)?,
    None => CommandAttributes {
      bang: false,
      nargs: CommandNargs::Zero,
    },
  };
  let options = match options.0 {
    Some(v) => options_from(&ctx, v)?,
    None => CommandOptions {
      force: true,
      alias: None,
    },
  };

  let definition = CommandDefinition {
    name: name.to_compact_string(),
    callback: CommandCallback::Rqjs(Persistent::save(&ctx, callback)),
    attributes,
    options,
  };

  let kernel = get_state(&ctx).ok_or_else(|| {
    Exception::throw_message(&ctx, "js kernel is not initialised")
  })?;
  let manager = kernel.borrow().editor.command_manager.clone();
  let mut manager = lock!(manager);
  match manager.insert(
    definition.name.clone(),
    CommandDefinition::to_rc(definition),
  ) {
    Ok(Some(replaced)) => definition_to_object(&ctx, &replaced),
    Ok(None) => Ok(Value::new_undefined(ctx.clone())),
    Err(e) => Err(Exception::throw_message(&ctx, &e.to_string())),
  }
}

/// `cmd.list()`.
pub fn list<'js>(ctx: Ctx<'js>) -> JsResult<Vec<String>> {
  let kernel = get_state(&ctx).ok_or_else(|| {
    Exception::throw_message(&ctx, "js kernel is not initialised")
  })?;
  let manager = kernel.borrow().editor.command_manager.clone();
  let manager = lock!(manager);
  Ok(manager.keys().map(|k| k.to_string()).collect())
}

/// `cmd.get(name)`.
pub fn get<'js>(ctx: Ctx<'js>, name: String) -> JsResult<Value<'js>> {
  let kernel = get_state(&ctx).ok_or_else(|| {
    Exception::throw_message(&ctx, "js kernel is not initialised")
  })?;
  let manager = kernel.borrow().editor.command_manager.clone();
  let manager = lock!(manager);
  match manager.get(&name) {
    Some(def) => definition_to_object(&ctx, &def),
    None => Ok(Value::new_undefined(ctx.clone())),
  }
}

/// `cmd.remove(name)`.
pub fn remove<'js>(ctx: Ctx<'js>, name: String) -> JsResult<Value<'js>> {
  let kernel = get_state(&ctx).ok_or_else(|| {
    Exception::throw_message(&ctx, "js kernel is not initialised")
  })?;
  let manager = kernel.borrow().editor.command_manager.clone();
  let mut manager = lock!(manager);
  match manager.remove(&name) {
    Some(def) => definition_to_object(&ctx, &def),
    None => Ok(Value::new_undefined(ctx.clone())),
  }
}

/// Serialises a definition the way `rsvim_macro::ToV8` does on the v8 side:
/// one flat object, lowerCamelCase keys, no nesting beyond the two sub-objects.
fn definition_to_object<'js>(
  ctx: &Ctx<'js>,
  def: &CommandDefinitionRc,
) -> JsResult<Value<'js>> {
  let obj = Object::new(ctx.clone())?;
  obj.set("name", def.name.to_string())?;

  // The callback round-trips: `prev.callback()` in a user script has to stay
  // callable, which is what `CommandDefinition`'s serialisation promises.
  match &def.callback {
    CommandCallback::Rqjs(persistent) => {
      let func = persistent.clone().restore(ctx)?;
      obj.set("callback", func)?;
    }
    #[cfg(feature = "js-v8")]
    CommandCallback::V8(_) => {
      obj.set("callback", Value::new_undefined(ctx.clone()))?;
    }
  }

  let attributes = Object::new(ctx.clone())?;
  attributes.set("bang", def.attributes.bang)?;
  attributes.set("nargs", def.attributes.nargs.to_string())?;
  obj.set("attributes", attributes)?;

  let options = Object::new(ctx.clone())?;
  options.set("force", def.options.force)?;
  if let Some(alias) = &def.options.alias {
    options.set("alias", alias.to_string())?;
  }
  obj.set("options", options)?;

  Ok(obj.into_value())
}

/// Installs the `cmd_*` built-ins on `internal`.
pub fn install<'js>(ctx: &Ctx<'js>, internal: &Object<'js>) -> JsResult<()> {
  set_function_to(ctx, internal, "cmd_echo", echo)?;
  set_function_to(ctx, internal, "cmd_create", create)?;
  set_function_to(ctx, internal, "cmd_list", list)?;
  set_function_to(ctx, internal, "cmd_get", get)?;
  set_function_to(ctx, internal, "cmd_remove", remove)?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use crate::js::command::attr::CommandNargs;
  use crate::js::rqjs::binding::install_bindings;
  use crate::js::rqjs::runtime;
  use crate::js::rqjs::runtime::JsKernel;
  use crate::js::rqjs::runtime::JsRuntimeOptions;
  use crate::js::rqjs::runtime::set_state;
  use crate::prelude::*;
  use crate::tests::js::make_editor;
  use crate::tests::log::init as test_log_init;
  use compact_str::ToCompactString;
  use rquickjs::AsyncContext;
  use rquickjs::AsyncRuntime;

  fn with_kernel<R>(
    f: impl FnOnce(rquickjs::Ctx<'_>, runtime::JsKernelRc) -> R,
  ) -> R {
    let editor = make_editor();
    let (master_tx, _master_rx) = tokio::sync::mpsc::unbounded_channel();
    let kernel = JsKernel::to_rc(JsKernel {
      options: JsRuntimeOptions::default(),
      master_tx,
      editor: editor.clone(),
      blocked_tracker: tokio_util::task::TaskTracker::new(),
    });
    let rt = AsyncRuntime::new().unwrap();
    let ctx = futures::executor::block_on(AsyncContext::full(&rt)).unwrap();
    futures::executor::block_on(ctx.with(move |ctx| {
      set_state(&ctx, kernel.clone()).unwrap();
      install_bindings(&ctx).unwrap();
      let out = f(ctx, kernel.clone());
      // A registered command owns a `Persistent` function handle, and QuickJS
      // aborts the process if one outlives its runtime. Production gets this
      // for free from `Rsvim`'s field order; a test keeps the editor past the
      // runtime, so it has to say so explicitly.
      let manager = kernel.borrow().editor.command_manager.clone();
      lock!(manager).clear();
      out
    }))
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn echo_reaches_the_message_history() {
    test_log_init();
    with_kernel(|ctx, kernel| {
      ctx
        .eval::<(), _>("__InternalRsvimGlobalObject.cmd_echo('hello')")
        .unwrap();
      let history = lock!(kernel.borrow().editor.cmdline_text)
        .message_history()
        .iter()
        .cloned()
        .collect::<Vec<String>>();
      assert_eq!(history, vec!["hello".to_string()]);
    });
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn echo_through_the_javascript_wrapper() {
    test_log_init();
    with_kernel(|ctx, kernel| {
      ctx
        .eval::<(), _>(
          "const cmd = __InternalRsvimGlobalObject; cmd.cmd_echo('wrapped');",
        )
        .unwrap();
      let history = lock!(kernel.borrow().editor.cmdline_text)
        .message_history()
        .iter()
        .cloned()
        .collect::<Vec<String>>();
      assert_eq!(history, vec!["wrapped".to_string()]);
    });
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn echo_without_a_kernel_is_a_javascript_error() {
    test_log_init();
    let rt = AsyncRuntime::new().unwrap();
    let ctx = futures::executor::block_on(AsyncContext::full(&rt)).unwrap();
    futures::executor::block_on(ctx.with(|ctx| {
      // Bindings without userdata: the call must raise, not panic.
      install_bindings(&ctx).unwrap();
      let result =
        ctx.eval::<(), _>("__InternalRsvimGlobalObject.cmd_echo('x')");
      assert!(result.is_err());
    }));
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn create_registers_a_command() {
    test_log_init();
    with_kernel(|ctx, kernel| {
      ctx
        .eval::<(), _>(
          "__InternalRsvimGlobalObject.cmd_create('write', () => 42,
             {bang: true, nargs: '1'}, {force: false, alias: 'w'})",
        )
        .unwrap();
      let manager = kernel.borrow().editor.command_manager.clone();
      let manager = lock!(manager);
      let def = manager
        .get("write")
        .expect("the command must be registered");
      assert_eq!(def.name, "write".to_compact_string());
      assert!(def.attributes.bang);
      assert_eq!(def.attributes.nargs, CommandNargs::One);
      assert!(!def.options.force);
      assert_eq!(def.options.alias, Some("w".to_compact_string()));
    });
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn create_without_options_uses_the_defaults() {
    test_log_init();
    with_kernel(|ctx, kernel| {
      // The javascript wrapper fills `{force: true}` in; here we call the raw
      // binding to prove the fallback matches.
      ctx
        .eval::<(), _>(
          "__InternalRsvimGlobalObject.cmd_create('bare', () => 0,
             undefined, undefined)",
        )
        .unwrap();
      let manager = kernel.borrow().editor.command_manager.clone();
      let manager = lock!(manager);
      let def = manager.get("bare").expect("the command must be registered");
      assert_eq!(def.attributes.nargs, CommandNargs::Zero);
      assert!(def.options.force);
      assert_eq!(def.options.alias, None);
    });
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn create_twice_returns_the_previous_definition() {
    test_log_init();
    with_kernel(|ctx, kernel| {
      ctx
        .eval::<(), _>(
          "const g = __InternalRsvimGlobalObject;
           g.cmd_create('twice', () => 'first', undefined, {force: true});
           globalThis.__prev = g.cmd_create('twice', () => 'second',
             undefined, {force: true});
           globalThis.__prevName = __prev.name;
           globalThis.__prevCallable = typeof __prev.callback;",
        )
        .unwrap();
      let name: String = ctx.eval("globalThis.__prevName").unwrap();
      assert_eq!(name, "twice");
      let kind: String = ctx.eval("globalThis.__prevCallable").unwrap();
      assert_eq!(kind, "function");
      // The first callback must still be reachable through the returned object.
      let called: String = ctx.eval("globalThis.__prev.callback()").unwrap();
      assert_eq!(called, "first");
      let manager = kernel.borrow().editor.command_manager.clone();
      assert_eq!(lock!(manager).len(), 1);
    });
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn create_without_force_rejects_a_duplicate() {
    test_log_init();
    with_kernel(|ctx, _kernel| {
      let result = ctx.eval::<(), _>(
        "const g = __InternalRsvimGlobalObject;
         g.cmd_create('dup', () => 0, undefined, {force: false});
         g.cmd_create('dup', () => 0, undefined, {force: false});",
      );
      assert!(result.is_err(), "the second registration must throw");
    });
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn list_get_and_remove_round_trip() {
    test_log_init();
    with_kernel(|ctx, kernel| {
      ctx
        .eval::<(), _>(
          "const g = __InternalRsvimGlobalObject;
           g.cmd_create('alpha', () => 'a');
           g.cmd_create('beta', () => 'b');
           globalThis.__names = g.cmd_list().join(',');
           globalThis.__beta = g.cmd_get('beta').name;
           globalThis.__missing = g.cmd_get('nope');
           globalThis.__removed = g.cmd_remove('alpha').name;
           globalThis.__after = g.cmd_list().join(',');",
        )
        .unwrap();
      let names: String = ctx.eval("globalThis.__names").unwrap();
      assert_eq!(names, "alpha,beta");
      let beta: String = ctx.eval("globalThis.__beta").unwrap();
      assert_eq!(beta, "beta");
      let missing: String = ctx.eval("String(globalThis.__missing)").unwrap();
      assert_eq!(missing, "undefined");
      let removed: String = ctx.eval("globalThis.__removed").unwrap();
      assert_eq!(removed, "alpha");
      let after: String = ctx.eval("globalThis.__after").unwrap();
      assert_eq!(after, "beta");
      let manager = kernel.borrow().editor.command_manager.clone();
      assert_eq!(lock!(manager).len(), 1);
    });
  }
}
