// use crate::cfg::path_cfg::PathConfig;
use crate::cli::CliOptions;
use crate::evloop::EventLoop;
use crate::prelude::*;
use crate::state::ops::Operation;
use assert_fs::prelude::*;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use std::cell::RefCell;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::channel;
use std::task::Poll;
use std::task::Waker;
use std::thread_local;
use std::time::Duration;

#[derive(Debug)]
pub struct TempConfigDir {
  pub home_dir: assert_fs::TempDir,
  pub xdg_config_home: assert_fs::TempDir,
  pub xdg_cache_home: assert_fs::TempDir,
  pub xdg_data_home: assert_fs::TempDir,
}

#[derive(Debug, Clone)]
pub struct TempPathConfig {
  pub home_dir: PathBuf,
  pub xdg_config_home: PathBuf,
  pub xdg_cache_home: PathBuf,
  pub xdg_data_home: PathBuf,
}

thread_local! {
  pub static TEMP_PATH_CONFIG: RefCell<Option<TempPathConfig>> = const { RefCell::new(None) };
}

impl TempConfigDir {
  pub fn create() -> Self {
    let temp_dirs = TempConfigDir {
      home_dir: assert_fs::TempDir::new().unwrap(),
      xdg_config_home: assert_fs::TempDir::new().unwrap(),
      xdg_cache_home: assert_fs::TempDir::new().unwrap(),
      xdg_data_home: assert_fs::TempDir::new().unwrap(),
    };

    let temp_path = TempPathConfig {
      home_dir: temp_dirs.home_dir.to_path_buf(),
      xdg_config_home: temp_dirs.xdg_config_home.to_path_buf(),
      xdg_cache_home: temp_dirs.xdg_cache_home.to_path_buf(),
      xdg_data_home: temp_dirs.xdg_data_home.to_path_buf(),
    };

    TEMP_PATH_CONFIG.with_borrow_mut(|p| *p = Some(temp_path));

    temp_dirs
  }
}

pub fn make_configs(sources: Vec<(&Path, &str)>) -> TempConfigDir {
  let tp = TempConfigDir::create();

  for (path, src) in sources.iter() {
    let path = tp.xdg_config_home.child("rsvim").child(path);
    path.touch().unwrap();
    std::fs::write(path, src).unwrap();
  }

  tp
}

pub fn make_home_configs(sources: Vec<(&Path, &str)>) -> TempConfigDir {
  let tp = TempConfigDir::create();

  for (path, src) in sources.iter() {
    let path = tp.home_dir.child(path);
    path.touch().unwrap();
    std::fs::write(path, src).unwrap();
  }

  tp
}

pub fn make_event_loop(
  terminal_cols: u16,
  terminal_rows: u16,
  cli_opts: CliOptions,
) -> EventLoop {
  EventLoop::mock_new(terminal_cols, terminal_rows, cli_opts).unwrap()
}

// const INTERVAL: Duration = Duration::from_micros(1);

#[derive(Debug)]
struct SharedWaker {
  pub waker: Option<Waker>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MockEvent {
  /// Normal keyboard event
  Event(Event),

  /// Sleep for a specific amount of time.
  SleepFor(Duration),
}

const CTRL_D: Event = Event::Key(KeyEvent::new_with_kind(
  KeyCode::Char('d'),
  KeyModifiers::CONTROL,
  KeyEventKind::Press,
));

#[derive(Debug)]
pub struct MockEventReader {
  rx: Receiver<IoResult<Event>>,
  shared_waker: Arc<Mutex<SharedWaker>>,
}

impl MockEventReader {
  pub fn new(events: Vec<MockEvent>) -> Self {
    let (tx, rx) = channel();
    let shared_waker = Arc::new(Mutex::new(SharedWaker { waker: None }));
    let cloned_shared_waker = shared_waker.clone();

    std::thread::spawn(move || {
      for (i, event) in events.iter().enumerate() {
        trace!("Send mock event[{i}]: {event:?}");

        match event {
          MockEvent::Event(e) => {
            // std::thread::sleep(INTERVAL);
            tx.send(Ok(e.clone())).unwrap();
          }
          MockEvent::SleepFor(d) => {
            std::thread::sleep(*d);
          }
        }

        let mut thread_shared_waker = lock!(cloned_shared_waker);
        if let Some(waker) = thread_shared_waker.waker.take() {
          waker.wake();
        }
      }

      trace!("Send final mock event[{}]: CTRL+D {CTRL_D:?}", events.len());
      // std::thread::sleep(INTERVAL);
      tx.send(Ok(CTRL_D.clone())).unwrap();

      let mut thread_shared_waker = lock!(cloned_shared_waker);
      if let Some(waker) = thread_shared_waker.waker.take() {
        waker.wake();
      }
    });

    Self { rx, shared_waker }
  }
}

impl futures::Stream for MockEventReader {
  type Item = IoResult<Event>;

  fn poll_next(
    self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
  ) -> Poll<Option<Self::Item>> {
    {
      let mut shared_waker = lock!(self.shared_waker);
      shared_waker.waker = Some(cx.waker().clone());
    }
    match self.rx.try_recv() {
      Ok(event) => Poll::Ready(Some(event)),
      _ => Poll::Pending,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MockOperation {
  /// Editor operation
  Operation(Operation),

  /// Sleep for a specific amount of time.
  SleepFor(Duration),

  Exit,
}

const EXIT: MockOperation = MockOperation::Exit;

#[derive(Debug)]
pub struct MockOperationReader {
  rx: Receiver<IoResult<MockOperation>>,
  shared_waker: Arc<Mutex<SharedWaker>>,
}

impl MockOperationReader {
  pub fn new(operations: Vec<MockOperation>) -> Self {
    let (tx, rx) = channel();
    let shared_waker = Arc::new(Mutex::new(SharedWaker { waker: None }));
    let cloned_shared_waker = shared_waker.clone();

    std::thread::spawn(move || {
      for (i, op) in operations.iter().enumerate() {
        trace!("Send mock operation[{i}]: {op:?}");

        match op {
          MockOperation::Operation(op) => {
            // std::thread::sleep(INTERVAL);
            tx.send(Ok(MockOperation::Operation(op.clone()))).unwrap();
          }
          MockOperation::SleepFor(d) => {
            std::thread::sleep(*d);
          }
          MockOperation::Exit => {
            // std::thread::sleep(INTERVAL);
            tx.send(Ok(MockOperation::Exit)).unwrap();
          }
        }

        let mut thread_shared_waker = lock!(cloned_shared_waker);
        if let Some(waker) = thread_shared_waker.waker.take() {
          waker.wake();
        }
      }

      trace!(
        "Send final mock operation[{}]: Exit {:?}",
        operations.len(),
        EXIT
      );
      // std::thread::sleep(INTERVAL);
      tx.send(Ok(EXIT.clone())).unwrap();

      let mut thread_shared_waker = lock!(cloned_shared_waker);
      if let Some(waker) = thread_shared_waker.waker.take() {
        waker.wake();
      }
    });

    Self { rx, shared_waker }
  }
}

impl futures::Stream for MockOperationReader {
  type Item = IoResult<MockOperation>;

  fn poll_next(
    self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
  ) -> Poll<Option<Self::Item>> {
    {
      let mut shared_waker = lock!(self.shared_waker);
      shared_waker.waker = Some(cx.waker().clone());
    }
    match self.rx.try_recv() {
      Ok(op) => Poll::Ready(Some(op)),
      _ => Poll::Pending,
    }
  }
}
