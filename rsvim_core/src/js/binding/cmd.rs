//! APIs for the `cmd` module.

use crate::js::binding::set_function_to;
use crate::js::cmdline::send_cmdline_message;
use crate::js::command::attr::CommandAttributes;
use crate::js::command::attr::CommandNargs;
use crate::js::command::def::CommandCallback;
use crate::js::command::def::CommandDefinition;
use crate::js::command::def::CommandDefinitionRc;
use crate::js::command::opt::CommandOptions;
use crate::js::runtime::kernel;
use crate::prelude::*;
use compact_str::ToCompactString;
use rquickjs::Coerced;
use rquickjs::Ctx;
use rquickjs::Exception;
use rquickjs::Function;
use rquickjs::Object;
use rquickjs::Result as JsResult;
use rquickjs::Value;
use rquickjs::function::Opt;
use std::str::FromStr;

/// `cmd.echo(message)`.
///
/// The javascript wrapper in `rsvim_core/js/cmd.min.js` only checks that the
/// argument is not nil, so `cmd.echo(1)` is legal and v8 turned it into `"1"`
/// through `to_rust_string_lossy`. A plain `String` parameter would instead
/// reject anything that is not already a string, which broke every caller that
/// echoes a number, so the coercion is spelled out here.
pub fn echo<'js>(ctx: Ctx<'js>, message: Coerced<String>) -> JsResult<()> {
  let kernel = kernel(&ctx)?;
  send_cmdline_message(&kernel.borrow().editor, message.0);
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
  value: Option<Value<'js>>,
) -> JsResult<CommandAttributes> {
  let Some(obj) = value.as_ref().and_then(|v| v.as_object()) else {
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
fn options_from(value: Option<Value<'_>>) -> JsResult<CommandOptions> {
  let Some(obj) = value.as_ref().and_then(|v| v.as_object()) else {
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
  let attributes = attributes_from(&ctx, attributes.0)?;
  let options = options_from(options.0)?;

  let definition = CommandDefinition {
    name: name.to_compact_string(),
    callback: CommandCallback::new(&ctx, callback),
    attributes,
    options,
  };

  let kernel = kernel(&ctx)?;
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
  let kernel = kernel(&ctx)?;
  let manager = kernel.borrow().editor.command_manager.clone();
  let manager = lock!(manager);
  Ok(manager.keys().map(|k| k.to_string()).collect())
}

/// `cmd.get(name)`.
pub fn get<'js>(ctx: Ctx<'js>, name: String) -> JsResult<Value<'js>> {
  let kernel = kernel(&ctx)?;
  let manager = kernel.borrow().editor.command_manager.clone();
  let manager = lock!(manager);
  match manager.get(name.as_str()) {
    Some(def) => definition_to_object(&ctx, def),
    None => Ok(Value::new_undefined(ctx.clone())),
  }
}

/// `cmd.remove(name)`.
pub fn remove<'js>(ctx: Ctx<'js>, name: String) -> JsResult<Value<'js>> {
  let kernel = kernel(&ctx)?;
  let manager = kernel.borrow().editor.command_manager.clone();
  let mut manager = lock!(manager);
  match manager.remove(name.as_str()) {
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
  let func = def.callback.restore(ctx)?;
  obj.set("callback", func)?;

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
