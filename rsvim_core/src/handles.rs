//! Shared handles to the editor's data.
//!
//! The editor's data (buffers, UI tree, managers, ...) is owned by `Arc`s and
//! reached from two places:
//!
//! - the event loop, i.e. [`Rsvim`](crate::evloop::Rsvim),
//! - the v8 callbacks, which can only reach the isolate slot, i.e.
//!   [`JsKernel`](crate::js::JsKernel).
//!
//! Both sides keep a clone of [`EditorHandles`], and all the clones point to
//! the very same `Arc`s.

use crate::buf::BufferManagerArc;
use crate::cli::CliOptions;
use crate::cmdltext::CmdlineTextArc;
use crate::hl::ColorSchemeManagerArc;
use crate::js::command::CommandManagerArc;
use crate::js::resource::ResourceTableArc;
use crate::syntax::SyntaxManagerArc;
use crate::ui::tree::TreeArc;

/// A one-time packaging of the editor's shared data.
///
/// `Rsvim` and `JsKernel` each hold a clone, pointing to the **same** `Arc`s.
/// It cannot be kept in a single place because v8 callbacks (`promise_reject_cb`,
/// `module_resolve_cb`, all the bindings) can only reach the kernel through
/// [`JsRuntime::state`](crate::js::JsRuntime::state), and have no way to reach
/// the outer `Rsvim`.
#[derive(Debug, Clone)]
pub struct EditorHandles {
  /// Command line options.
  pub cli_opts: CliOptions,
  /// Widget tree for UI.
  pub tree: TreeArc,
  /// Vim buffers.
  pub buffer_manager: BufferManagerArc,
  /// Cmdline text.
  pub cmdline_text: CmdlineTextArc,
  /// Syntax and parsers.
  pub syntax_manager: SyntaxManagerArc,
  /// Colorschemes.
  pub colorscheme_manager: ColorSchemeManagerArc,
  /// Vim commands.
  pub command_manager: CommandManagerArc,
  /// Resources.
  pub resource_table: ResourceTableArc,
}
