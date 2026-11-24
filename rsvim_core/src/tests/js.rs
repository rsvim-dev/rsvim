use crate::buf::BufferManager;
use crate::cli::CliOptions;
use crate::cmdltext::CmdlineText;
use crate::handles::EditorHandles;
use crate::hl::ColorSchemeManager;
use crate::js::JsKernelRc;
use crate::js::JsRuntimeOptions;
use crate::js::command::CommandManager;
use crate::js::resource::ResourceTable;
use crate::prelude::*;
use crate::syntax::SyntaxManager;
use crate::ui::tree::Tree;
use std::sync::Arc;
use taffy::Style;
use tokio::sync::mpsc::unbounded_channel;

/// A v8 isolate together with its kernel, for the unit tests that need a raw
/// `v8::scope_with_context!` without running the editor's event loop.
pub struct TestJsRuntime {
  pub isolate: v8::OwnedIsolate,
  pub kernel: JsKernelRc,
}

impl TestJsRuntime {
  /// Opens the sand-boxed context.
  pub fn context(&self) -> v8::Global<v8::Context> {
    self.kernel.borrow().context.clone()
  }

  /// Runs a single tick of the js event loop.
  pub fn tick_event_loop(&mut self) {
    let context = self.context();
    v8::scope_with_context!(scope, &mut self.isolate, context);
    crate::js::checkpoint(scope);
  }
}

pub fn make_js_runtime() -> TestJsRuntime {
  let canvas_size = size!(10, 10);
  let (master_tx, _master_rx) = unbounded_channel();

  let cli_opts = CliOptions::new(false, vec![]);

  let style = Style {
    size: taffy::Size {
      width: taffy::prelude::length(canvas_size.width()),
      height: taffy::prelude::length(canvas_size.height()),
    },
    ..Default::default()
  };
  let tree = Tree::to_arc(Tree::new(style).unwrap());
  let syntax_manager = SyntaxManager::to_arc(SyntaxManager::new());
  let colorscheme_manager =
    ColorSchemeManager::to_arc(ColorSchemeManager::new());
  let buffers_manager = BufferManager::to_arc(BufferManager::new(
    Arc::downgrade(&syntax_manager),
    Arc::downgrade(&colorscheme_manager),
  ));
  let cmdline_text = CmdlineText::to_arc(CmdlineText::new(canvas_size));
  let ex_commands_manager = CommandManager::to_arc(CommandManager::default());
  let resource_table = ResourceTable::to_arc(ResourceTable::new());

  let editor = EditorHandles {
    cli_opts,
    tree,
    buffer_manager: buffers_manager,
    cmdline_text,
    syntax_manager,
    colorscheme_manager,
    command_manager: ex_commands_manager,
    resource_table,
  };

  let (isolate, kernel) =
    crate::js::new_isolate(JsRuntimeOptions::default(), master_tx, editor);
  TestJsRuntime { isolate, kernel }
}
