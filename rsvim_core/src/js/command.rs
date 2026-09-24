//! Vim ex commands.

pub mod attr;
pub mod ctx;
pub mod def;
pub mod opt;

#[cfg(test)]
mod attr_tests;
#[cfg(test)]
mod opt_tests;

use crate::buf::unicode::char_is_whitespace;
use crate::chan::ExCommandReq;
use crate::js::JsFuture;
use crate::js::JsRuntime;
use crate::js::TaskId;
use crate::js::command::ctx::CommandContext;
use crate::js::command::ctx::CommandContextBuilder;
use crate::js::converter::*;
use crate::js::execute_module;
use crate::prelude::*;
use compact_str::CompactString;
use compact_str::ToCompactString;
use def::CommandDefinitionRc;
use itertools::Itertools;

const JS_COMMAND_NAME: &str = "js";

#[derive(Debug, Clone)]
/// Builtin `:js` command
pub struct CommandFuture {
  pub task_id: TaskId,
  pub context: CommandContext,
  pub is_builtin_js: bool,
  pub definition: Option<CommandDefinitionRc>,
}

impl JsFuture for CommandFuture {
  fn run(&mut self, scope: &mut v8::PinScope) {
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
      execute_module(scope, &filename, Some(self.context.args[0].trim()));
    } else {
      let def = self.definition.clone().unwrap();
      let undefined = v8::undefined(scope).into();
      let callback = v8::Local::new(scope, (*def.callback).clone());
      let args: Vec<v8::Local<v8::Value>> = vec![self.context.to_v8(scope)];

      v8::tc_scope!(let tc_scope, scope);

      callback.call(tc_scope, undefined, &args);

      // Report if callback threw an exception.
      if tc_scope.has_caught() {
        let exception = tc_scope.exception().unwrap();
        let exception = v8::Global::new(tc_scope, exception);
        let state_rc = JsRuntime::state(tc_scope);
        state_rc
          .borrow_mut()
          .exceptions
          .capture_exception(exception);
      }
    }
  }
}

#[derive(Debug, Default, rsvim_macro::ArcMutexPtr)]
pub struct CommandManager {
  // Maps from command "name" to its "definition".
  commands: BTreeMap<CompactString, CommandDefinitionRc>,

  // Maps from command "alias" to its "name".
  aliases: FoldMap<CompactString, CompactString>,
}

pub type CommandManagerKeys<'a> =
  std::collections::btree_map::Keys<'a, CompactString, CommandDefinitionRc>;
pub type CommandManagerValues<'a> =
  std::collections::btree_map::Values<'a, CompactString, CommandDefinitionRc>;
pub type CommandManagerIter<'a> =
  std::collections::btree_map::Iter<'a, CompactString, CommandDefinitionRc>;

impl CommandManager {
  pub fn is_empty(&self) -> bool {
    self.commands.is_empty()
  }

  pub fn len(&self) -> usize {
    self.commands.len()
  }

  pub fn remove(&mut self, name: &str) -> Option<CommandDefinitionRc> {
    self.commands.remove(name)
  }

  /// Insert new command definition.
  ///
  /// Every "command" has a unique name and it alias (if exists). When
  /// inserts/registers a new command, both its name and alias cannot conflict
  /// with existing registered ones.
  ///
  /// # Returns
  ///
  /// 1. It returns `Ok(None)` if registered successfully, and no conflicting
  ///    one exists.
  /// 2. It returns `Ok(CommandDefinitionRc)` if registered successfully, and
  ///    previous one is been removed and returned. Note: this requires the
  ///    `force` option.
  /// 3. It returns `Err` if registered failed, because either command name or
  ///    alias already exists, and user doesn't have the `force` option.
  pub fn insert(
    &mut self,
    name: CompactString,
    definition: CommandDefinitionRc,
  ) -> TheResult<Option<CommandDefinitionRc>> {
    let alias = definition.options.alias.clone();

    if !definition.options.force {
      if self.commands.contains_key(&name) {
        return Err(TheErr::CommandAlreadyExist(name));
      }
      if let Some(ref alias) = alias
        && self.aliases.contains_key(alias.as_str())
      {
        return Err(TheErr::CommandAlreadyExist(alias.clone()));
      }
    }

    if let Some(alias) = alias {
      self.aliases.insert(alias, name.clone());
    }

    let maybe_old = self.commands.insert(name, definition);
    Ok(maybe_old)
  }

  pub fn get(&self, name: &str) -> Option<CommandDefinitionRc> {
    self.commands.get(name).cloned()
  }

  pub fn contains_key(&self, name: &str) -> bool {
    self.commands.contains_key(name)
  }

  pub fn keys(&self) -> CommandManagerKeys<'_> {
    self.commands.keys()
  }

  pub fn values(&self) -> CommandManagerValues<'_> {
    self.commands.values()
  }

  pub fn iter(&self) -> CommandManagerIter<'_> {
    self.commands.iter()
  }

  pub fn first_key_value(
    &self,
  ) -> Option<(&CompactString, &CommandDefinitionRc)> {
    self.commands.first_key_value()
  }

  pub fn last_key_value(
    &self,
  ) -> Option<(&CompactString, &CommandDefinitionRc)> {
    self.commands.last_key_value()
  }
}

impl CommandManager {
  pub fn parse(&self, req: &ExCommandReq) -> Option<CommandFuture> {
    debug_assert_eq!(req.payload.trim(), req.payload);

    let mut context = CommandContextBuilder::default();
    context.current_buffer_id(req.current_buf_id);
    context.current_window_id(req.current_win_id);

    let (mut name, body) = match req.payload.find(char_is_whitespace) {
      Some(pos) => {
        let name = req.payload.get(0..pos).unwrap().trim().to_compact_string();
        let body = req.payload.get(pos..).unwrap().to_compact_string();
        (name, body)
      }
      None => {
        let name = req.payload.trim().to_compact_string();
        let body = "".to_compact_string();
        (name, body)
      }
    };

    if name.ends_with("!") {
      let _last = name.pop();
      debug_assert_eq!(_last, Some('!'));
      context.bang(true);
    }

    let is_builtin_js = name == JS_COMMAND_NAME;
    let task_id = TaskId::next();

    if is_builtin_js {
      // For builtin js command, it:
      // - Has only 1 args, which is the js expression payload
      // - Doesn't have a js function based command definition

      debug_assert!(!self.commands.contains_key(&name));
      let args = vec![body];
      context.args(args);
      let context = context.build().unwrap();

      Some(CommandFuture {
        task_id,
        context,
        is_builtin_js,
        definition: None,
      })
    } else if self.commands.contains_key(&name)
      || self.aliases.contains_key(&name)
    {
      // For user registered commands, it can have:
      // - Command alias
      // - Command arguments split by whitespaces
      // - Js function based command definition

      let name = self.aliases.get(&name).unwrap_or(&name).clone();
      debug_assert!(self.commands.contains_key(&name));
      let args = body
        .split(char_is_whitespace)
        .map(|a| a.to_compact_string())
        .collect_vec();
      context.args(args);
      let context = context.build().unwrap();
      let definition = Some(self.commands.get(&name).unwrap().clone());

      Some(CommandFuture {
        task_id,
        context,
        is_builtin_js,
        definition,
      })
    } else {
      None
    }
  }
}
