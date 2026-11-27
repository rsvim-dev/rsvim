//! Ex command definition.

use crate::js::command::attr::CommandAttributes;
use crate::js::command::opt::CommandOptions;
use compact_str::CompactString;

/// The javascript function a user registered with `cmd.create(name, cb)`.
///
/// QuickJS hands out [`rquickjs::Persistent`] for "a callable that outlives the
/// current scope": a `rquickjs::Function` dies with its [`rquickjs::Ctx`].
///
/// NOTE: a `Persistent` that outlives its runtime makes QuickJS abort the
/// process on drop. The command manager lives in [`crate::evloop::Editor`],
/// which the runtime does not own, so no field order can decide which drops
/// first; `impl Drop for` [`crate::js::JsHost`] clears the manager before the
/// runtime goes away.
#[derive(Clone)]
pub struct CommandCallback(rquickjs::Persistent<rquickjs::Function<'static>>);

impl std::fmt::Debug for CommandCallback {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("CommandCallback")
  }
}

impl CommandCallback {
  /// Wraps a freshly created javascript function.
  pub fn new<'js>(
    ctx: &rquickjs::Ctx<'js>,
    callback: rquickjs::Function<'js>,
  ) -> Self {
    Self(rquickjs::Persistent::save(ctx, callback))
  }

  /// Brings the handle back to life inside `ctx`.
  pub fn restore<'js>(
    &self,
    ctx: &rquickjs::Ctx<'js>,
  ) -> rquickjs::Result<rquickjs::Function<'js>> {
    self.0.clone().restore(ctx)
  }
}

#[derive_where::derive_where(Debug)]
#[derive(Clone, rsvim_macro::RcPtr)]
pub struct CommandDefinition {
  pub name: CompactString,
  #[derive_where(skip)]
  pub callback: CommandCallback,
  pub attributes: CommandAttributes,
  pub options: CommandOptions,
}
