//! TUI utility.

use crate::prelude::*;
use crossterm::event::DisableFocusChange;
use crossterm::event::DisableMouseCapture;
use crossterm::event::EnableFocusChange;
use crossterm::event::EnableMouseCapture;
use crossterm::execute;
use std::io::Write;

/// Initialize terminal raw mode.
pub fn initialize_raw_mode() -> IoResult<()> {
  if !crossterm::terminal::is_raw_mode_enabled()? {
    crossterm::terminal::enable_raw_mode()?;
  }

  let mut out = std::io::stdout();
  execute!(
    out,
    crossterm::terminal::EnterAlternateScreen,
    crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
    EnableMouseCapture,
    EnableFocusChange,
  )?;
  Ok(())
}

/// Shutdown terminal raw mode.
pub fn shutdown_raw_mode() -> IoResult<()> {
  let mut out = std::io::stdout();
  execute!(
    out,
    DisableMouseCapture,
    DisableFocusChange,
    crossterm::terminal::LeaveAlternateScreen,
  )?;

  if crossterm::terminal::is_raw_mode_enabled()? {
    crossterm::terminal::disable_raw_mode()?;
  }

  Ok(())
}

/// Shutdown terminal raw mode when panic, and dump backtrace.
pub fn shutdown_raw_mode_on_panic() {
  std::panic::set_hook(Box::new(|panic_hook_info| {
    // Recover terminal mode.
    if shutdown_raw_mode().is_err() {
      eprintln!("FATAL! Failed to recover terminal!");
    }

    let now = jiff::Zoned::now();
    let btrace = std::backtrace::Backtrace::force_capture();
    println!("FATAL! Rsvim panics at {now}!");
    println!("{panic_hook_info:?}");
    println!("{btrace}");
    let log_name = format!(
      "rsvim_coredump_{:0>4}-{:0>2}-{:0>2}_{:0>2}-{:0>2}-{:0>2}-{:0>3}.log",
      now.date().year(),
      now.date().month(),
      now.date().day(),
      now.time().hour(),
      now.time().minute(),
      now.time().second(),
      now.time().millisecond(),
    );
    let log_path = Path::new(log_name.as_str());
    if let Ok(mut f) = std::fs::File::create(log_path) {
      if f
        .write_all(
          format!("FATAL! Rsvim panics!\n{panic_hook_info:?}\n{btrace}")
            .as_bytes(),
        )
        .is_err()
      {
        eprintln!("FATAL! Failed to write rsvim coredump!");
      }
    } else {
      eprintln!("FATAL! Failed to create rsvim coredump!");
    }
  }));
}
