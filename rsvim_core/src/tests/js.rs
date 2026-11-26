use crate::buf::BufferManager;
use crate::chan::JsMessage;
use crate::chan::MasterMessage;
use crate::cli::CliOptions;
use crate::cmdltext::CmdlineText;
use crate::evloop::Editor;
use crate::hl::ColorSchemeManager;
use crate::js::JsHost;
use crate::js::JsRuntimeOptions;
use crate::js::command::CommandManager;
use crate::js::resource::ResourceTable;
use crate::prelude::*;
use crate::syntax::SyntaxManager;
use crate::ui::tree::Tree;
use rquickjs::Ctx;
use std::sync::Arc;
use std::time::Duration;
use taffy::Style;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::task::TaskTracker;

/// Builds an [`Editor`] whose widgets exist but whose TUI never starts.
///
/// Engine-neutral on purpose: the unit tests need a kernel, and a kernel needs
/// an editor. Constructing it in one place keeps every test from drifting apart.
pub fn make_editor() -> Editor {
  let canvas_size = size!(10, 10);
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
  let buffer_manager = BufferManager::to_arc(BufferManager::new(
    Arc::downgrade(&syntax_manager),
    Arc::downgrade(&colorscheme_manager),
  ));
  let cmdline_text = CmdlineText::to_arc(CmdlineText::new(canvas_size));
  let command_manager = CommandManager::to_arc(CommandManager::default());
  let resource_table = ResourceTable::to_arc(ResourceTable::new());

  Editor {
    cli_opts: CliOptions::new(false, vec![]),
    tree,
    buffer_manager,
    cmdline_text,
    syntax_manager,
    colorscheme_manager,
    command_manager,
    resource_table,
  }
}

/// A [`JsHost`] together with the receiving ends of its channels, for the unit
/// tests that need to run javascript without the editor's event loop.
///
/// [`TestJs::run_until`] is the whole event loop in miniature: it waits for the
/// two things that can make javascript run later -- a finished operation and a
/// due timer -- and hands each to the host, exactly as `Rsvim` does.
pub struct TestJs {
  pub host: JsHost,
  /// The module loader reads the (temporary) config directories, so they have
  /// to exist for as long as the host does.
  _config: crate::tests::evloop::TempConfigDir,
  pub editor: Editor,
  pub jsrt_rx: UnboundedReceiver<JsMessage>,
  pub master_rx: UnboundedReceiver<MasterMessage>,
}

impl TestJs {
  /// Runs `f` inside the javascript context, through the host's entrance.
  pub fn enter<R>(&mut self, f: impl for<'js> FnOnce(Ctx<'js>) -> R) -> R {
    self.host.enter(f)
  }

  /// Evaluates a script and returns its value.
  pub fn eval<T: for<'js> rquickjs::FromJs<'js>>(&mut self, src: &str) -> T {
    self.host.enter(|ctx| ctx.eval(src).unwrap())
  }

  /// Evaluates a script and discards its value.
  pub fn run(&mut self, src: &str) {
    self.host.enter(|ctx| {
      let _: rquickjs::Value = ctx.eval(src).unwrap();
    });
  }

  /// Evaluates `expr`, which must be a promise, and runs the loop until it
  /// settles. The settled value is left in `globalThis.__value`.
  pub async fn settle(&mut self, expr: &str) -> Result<(), String> {
    self.run(&format!(
      "globalThis.__state = 0; globalThis.__value = undefined;
       ({expr}).then(
         (v) => {{ globalThis.__value = v; globalThis.__state = 1; }},
         (e) => {{ globalThis.__value = e; globalThis.__state = 2; }});"
    ));
    let done = self
      .run_until(Duration::from_secs(5), |js| {
        js.eval::<i32>("globalThis.__state") != 0
      })
      .await;
    assert!(done, "the promise never settled: {expr}");
    if self.eval::<i32>("globalThis.__state") == 1 {
      Ok(())
    } else {
      Err(self.eval::<String>(
        "String(globalThis.__value && globalThis.__value.message || globalThis.__value)",
      ))
    }
  }

  /// The messages the editor's command line received so far.
  pub fn messages(&self) -> Vec<String> {
    lock!(self.editor.cmdline_text)
      .message_history()
      .iter()
      .cloned()
      .collect()
  }

  /// Waits for events and feeds them to the host until `done` is true or
  /// `limit` has passed. Returns whether `done` became true.
  pub async fn run_until(
    &mut self,
    limit: Duration,
    mut done: impl FnMut(&mut Self) -> bool,
  ) -> bool {
    let end = tokio::time::Instant::now() + limit;
    loop {
      if done(self) {
        return true;
      }
      tokio::select! {
        msg = self.jsrt_rx.recv() => match msg {
          Some(JsMessage::OpDone(completion)) => self.host.complete(completion),
          Some(JsMessage::ExCommandReq(_)) => unreachable!("no editor here"),
          None => return done(self),
        },
        Some(timer) = std::future::poll_fn(|cx| self.host.poll_timer(cx)) => {
          self.host.run_timer(timer);
        }
        _ = tokio::time::sleep_until(end) => return done(self),
      }
    }
  }

  /// Runs the loop for `duration`.
  pub async fn run_for(&mut self, duration: Duration) {
    self.run_until(duration, |_| false).await;
  }
}

/// Builds a [`TestJs`] with a fresh editor.
///
/// Must be called from inside a tokio runtime if the test is going to spawn
/// asynchronous operations.
pub fn make_js_host() -> TestJs {
  make_js_host_with(JsRuntimeOptions::default())
}

/// [`make_js_host`] with explicit runtime options.
pub fn make_js_host_with(options: JsRuntimeOptions) -> TestJs {
  let config = crate::tests::evloop::TempConfigDir::create();
  let (master_tx, master_rx) = unbounded_channel();
  let (jsrt_tx, jsrt_rx) = unbounded_channel();
  let editor = make_editor();
  let host = JsHost::new(
    options,
    master_tx,
    jsrt_tx,
    editor.clone(),
    TaskTracker::new(),
    TaskTracker::new(),
  )
  .unwrap();
  TestJs {
    host,
    _config: config,
    editor,
    jsrt_rx,
    master_rx,
  }
}
