use crate::buf::BufferManager;
use crate::cli::CliOptions;
use crate::cmdltext::CmdlineText;
use crate::hl::ColorSchemeManager;
use crate::js::JsRuntime;
use crate::js::JsRuntimeOptions;
use crate::js::command::CommandManager;
use crate::js::resource::ResourceTable;
use crate::prelude::*;
use crate::syntax::SyntaxManager;
use crate::ui::tree::Tree;
use std::sync::Arc;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use taffy::Style;
use tokio::sync::mpsc::unbounded_channel;
use tokio::time::Instant;

pub fn make_js_runtime() -> JsRuntime {
  let canvas_size = size!(10, 10);
  let (master_tx, _master_rx) = unbounded_channel();
  let (_jsrt_tx, jsrt_rx) = unbounded_channel();

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

  let startup_moment = Instant::now();
  let startup_unix_epoch = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_millis();

  JsRuntime::new(
    JsRuntimeOptions::default(),
    startup_moment,
    startup_unix_epoch,
    master_tx,
    jsrt_rx,
    cli_opts,
    tree,
    buffers_manager,
    cmdline_text,
    syntax_manager,
    colorscheme_manager,
    ex_commands_manager,
    resource_table,
  )
}
