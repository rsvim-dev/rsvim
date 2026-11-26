//! Vim ex commands.

pub mod attr;
pub mod ctx;
pub mod def;
pub mod opt;

#[cfg(test)]
mod attr_tests;
#[cfg(test)]
mod opt_tests;
#[cfg(test)]
mod tests;

use crate::buf::unicode::char_is_whitespace;
use crate::chan::ExCommandReq;
use crate::js::JsHost;
use crate::js::TaskId;
use crate::js::command::ctx::CommandContext;
use crate::js::command::ctx::CommandContextBuilder;
use crate::js::err::JsError;
use crate::prelude::*;
use compact_str::CompactString;
use compact_str::ToCompactString;
use def::CommandDefinitionRc;
use itertools::Itertools;
use rquickjs::Ctx;
use rquickjs::Object;

const JS_COMMAND_NAME: &str = "js";

/// A parsed `:Ex` command that is ready to run.
///
/// This used to implement the v8 backend's `JsFuture` trait, because v8 needed
/// a queue of pending callbacks that the tick drained. There is no queue now:
/// the event loop calls [`CommandFuture::run`] directly, which enters the
/// javascript host.
#[derive(Debug, Clone)]
pub struct CommandFuture {
  /// The identifier of this command invocation.
  pub task_id: TaskId,
  /// The context the callback receives.
  pub context: CommandContext,
  /// Whether this is the builtin `:js` command.
  pub is_builtin_js: bool,
  /// The user-defined command, absent for the builtin one.
  pub definition: Option<CommandDefinitionRc>,
}

impl CommandFuture {
  /// Runs the command and returns whatever it threw, if anything.
  ///
  /// The caller reports the error: a [`CommandFuture`] does not know the
  /// editor's message history, and threading it in would put an `Arc` chain
  /// through a value that is built while parsing a keystroke.
  ///
  /// An `async` callback that rejects does not surface here: its rejection is
  /// reported by the host once the job queue is drained, like any other
  /// unhandled rejection.
  pub fn run(&self, host: &mut JsHost) -> Option<JsError> {
    trace!(
      "|CommandFuture| command name:{:?}({:?})",
      self
        .definition
        .as_ref()
        .map(|def| def.name.clone())
        .unwrap_or("?".to_compact_string()),
      self.task_id
    );

    if self.is_builtin_js {
      let filename = format!("<command-js:{}>", self.task_id);
      debug_assert_eq!(self.context.args.len(), 1);
      return match host
        .execute_module(&filename, Some(self.context.args[0].trim()))
      {
        Ok(()) => None,
        Err(TheErr::JsError(err)) => Some(*err),
        Err(e) => Some(JsError::unlocated(e.to_string())),
      };
    }

    let def = self.definition.clone()?;

    host.enter(|ctx| -> Option<JsError> {
      let callback = match def.callback.restore(&ctx) {
        Ok(callback) => callback,
        Err(e) => return Some(JsError::from_error(&ctx, e)),
      };
      let arg = match self.context.to_object(&ctx) {
        Ok(arg) => arg,
        Err(e) => return Some(e),
      };
      match callback.call::<_, ()>((arg,)) {
        Ok(()) => None,
        Err(e) => Some(JsError::from_error(&ctx, e)),
      }
    })
  }
}

impl CommandContext {
  /// Converts the context into the object a command callback receives.
  ///
  /// The field names are lowerCamelCase for the same reason the v8 backend's
  /// `ToV8` derive produced them: the javascript side reads
  /// `ctx.currentBufferId` (see `rsvim_core/src/js/command/tests.rs`).
  pub fn to_object<'js>(&self, ctx: &Ctx<'js>) -> Result<Object<'js>, JsError> {
    let build = || -> rquickjs::Result<Object<'js>> {
      let object = Object::new(ctx.clone())?;
      object.set("bang", self.bang)?;
      let args: Vec<String> = self.args.iter().map(|a| a.to_string()).collect();
      object.set("args", args)?;
      object.set("currentBufferId", i32::from(self.current_buffer_id))?;
      object.set("currentWindowId", i32::from(self.current_window_id))?;
      Ok(object)
    };
    build().map_err(JsError::from_js_error)
  }
}

#[derive(Debug, Default, rsvim_macro::ArcMutexPtr)]
/// Registry of `:Ex` commands.
///
/// A definition holds a javascript callback handle. In QuickJS that handle is a
/// [`rquickjs::Persistent`], and a persistent that outlives its runtime makes
/// the runtime abort when it is dropped -- hence [`CommandManager::clear`],
/// which `impl Drop for` [`crate::js::JsHost`] calls before the runtime goes
/// away.
pub struct CommandManager {
  commands: BTreeMap<CompactString, CommandDefinitionRc>,
  aliases: FoldMap<CompactString, CompactString>,
}

/// Iterator type of the ex commands.
pub type CommandManagerIter<'a> =
  std::collections::btree_map::Iter<'a, CompactString, CommandDefinitionRc>;

/// Iterator type of the ex command names.
pub type CommandManagerKeys<'a> =
  std::collections::btree_map::Keys<'a, CompactString, CommandDefinitionRc>;

/// Iterator type of the ex command definitions.
pub type CommandManagerValues<'a> =
  std::collections::btree_map::Values<'a, CompactString, CommandDefinitionRc>;

impl CommandManager {
  /// Checks if the commands collection is empty.
  pub fn is_empty(&self) -> bool {
    self.commands.is_empty()
  }

  /// Gets the number of commands.
  pub fn len(&self) -> usize {
    self.commands.len()
  }

  /// Removes the command named `key` from this [`CommandManager`].
  ///
  /// This operation also removes its alias.
  pub fn remove(&mut self, key: &str) -> Option<CommandDefinitionRc> {
    let key = key.to_compact_string();
    let def = self.commands.remove(&key);
    if let Some(def) = def.as_ref() {
      if let Some(alias) = def.options.alias.clone() {
        self.aliases.remove(&alias);
      }
    }
    def
  }

  /// Removes every command and alias.
  ///
  /// Dropping the definitions drops their [`rquickjs::Persistent`] handles. In
  /// QuickJS a persistent must be dropped before the runtime, otherwise
  /// `JS_FreeRuntime` asserts. [`crate::js::JsHost`] calls this when it is
  /// dropped, on every path out.
  pub fn clear(&mut self) {
    self.commands.clear();
    self.aliases.clear();
  }

  /// Adds a new command to this [`CommandManager`].
  ///
  /// - Ok(None): the command does not exist yet.
  /// - Ok(Some(def)): the command exists but will be replaced by the new
  ///   definition, `def` is the old one. Note this could be dangerous to
  ///   replace a command with a same name, unless the new definition has the
  ///   `force` option.
  /// - Err(_): the command conflicts with an existing one.
  pub fn insert(
    &mut self,
    name: CompactString,
    definition: CommandDefinitionRc,
  ) -> TheResult<Option<CommandDefinitionRc>> {
    trace!("|insert| name:{:?}, definition:{:?}", name, definition);
    debug_assert_eq!(name, definition.name.clone());

    // Check conflicts
    if self.commands.contains_key(&name) && !definition.options.force {
      return Err(TheErr::CommandAlreadyExist(name));
    }

    // Check the alias
    if let Some(alias) = definition.options.alias.clone() {
      if self.aliases.contains_key(&alias) {
        if !definition.options.force {
          return Err(TheErr::CommandAlreadyExist(alias));
        }
        self.aliases.remove(&alias);
      }
    }

    let replaced = self.commands.insert(name.clone(), definition.clone());
    if let Some(alias) = definition.options.alias.clone() {
      self.aliases.insert(alias, name);
    }
    Ok(replaced)
  }

  /// Gets a reference to the command, by name.
  pub fn get(&self, name: &str) -> Option<&CommandDefinitionRc> {
    self.commands.get(&name.to_compact_string())
  }

  /// Checks if the command is inside this [`CommandManager`], by name.
  pub fn contains_key(&self, name: &str) -> bool {
    self.commands.contains_key(&name.to_compact_string())
  }

  /// Iterates all the commands.
  pub fn keys(&self) -> CommandManagerKeys<'_> {
    self.commands.keys()
  }

  /// Iterates all the commands.
  pub fn values(&self) -> CommandManagerValues<'_> {
    self.commands.values()
  }

  /// Iterates all the commands.
  pub fn iter(&self) -> CommandManagerIter<'_> {
    self.commands.iter()
  }

  /// Iterates all the commands, but by key-value.
  pub fn first_key_value(
    &self,
  ) -> Option<(&CompactString, &CommandDefinitionRc)> {
    self.commands.first_key_value()
  }

  /// Iterates all the commands.
  pub fn last_key_value(
    &self,
  ) -> Option<(&CompactString, &CommandDefinitionRc)> {
    self.commands.last_key_value()
  }

  /// Parses the `Ex` command request into a [`CommandFuture`].
  ///
  /// Returns the parsed command, referring to the builtin `:js`, or any
  /// user-defined commands.
  pub fn parse(&self, req: &ExCommandReq) -> Option<CommandFuture> {
    debug_assert_eq!(req.payload.trim(), req.payload);
    trace!("|parse| payload:{:?}", req.payload);

    let mut context = CommandContextBuilder::default();
    context.current_buffer_id(req.current_buf_id);
    context.current_window_id(req.current_win_id);

    let mut name = req.payload.to_compact_string();
    let body = if let Some(index) = req.payload.find(char_is_whitespace) {
      name = req.payload[0..index].to_compact_string();
      req.payload[index..].trim().to_compact_string()
    } else {
      name.clone()
    };
    // The trailing `!` is never part of the name.
    if name.ends_with("!") {
      name.pop();
      context.bang(true);
    }

    let task_id = TaskId::next();

    if name == JS_COMMAND_NAME {
      debug_assert!(!self.commands.contains_key(&name));
      context.args(vec![body]);
      return Some(CommandFuture {
        task_id,
        context: context.build().unwrap(),
        is_builtin_js: true,
        definition: None,
      });
    }

    // Resolve the alias, then look the command up by its real name.
    let resolved = if self.commands.contains_key(&name) {
      name.clone()
    } else {
      self.aliases.get(&name)?.clone()
    };
    let definition = self.commands.get(&resolved)?.clone();

    let args: Vec<CompactString> = if body.is_empty() {
      vec![]
    } else {
      body
        .split(char_is_whitespace)
        .map(|a| a.to_compact_string())
        .collect_vec()
    };
    context.args(args);

    Some(CommandFuture {
      task_id,
      context: context.build().unwrap(),
      is_builtin_js: false,
      definition: Some(definition),
    })
  }
}
