//! Loggings for testing.
//!
//! NOTE: This module should only be used in unit tests, not some where else.

use crate::log::RSVIM_LOG;
use std::sync::Once;

/// Initialize the logging prints to `stderr`.
pub fn init() {
  static INITIALIZED: Once = Once::new();
  INITIALIZED.call_once(|| {
    let filter = env_filter::Builder::from_env(RSVIM_LOG).build();
    let formatter = "%T%.3f";

    fern::Dispatch::new()
      .filter(move |metadata| filter.enabled(metadata))
      .format(|out, message, record| {
        out.finish(format_args!(
          "{} {:<5} {}:{}| {}",
          jiff::Zoned::now().strftime(formatter),
          record.level(),
          record.target(),
          record.line().unwrap_or(0),
          message
        ))
      })
      .chain(std::io::stdout())
      .apply()
      .unwrap();

    // // tracing for oxc_resolver
    // let subscriber = tracing_subscriber::FmtSubscriber::builder()
    //   // .with_file(true)
    //   .with_line_number(true)
    //   // .with_thread_ids(true)
    //   // .with_thread_names(true)
    //   .with_level(true)
    //   .with_ansi(false)
    //   .with_env_filter(tracing_subscriber::EnvFilter::from_env(RSVIM_LOG))
    //   .with_writer(std::io::stdout)
    //   .finish();
    // tracing::subscriber::set_global_default(subscriber).unwrap();

    // log::info!("GITHUB_ACTIONS:{:?}", std::env::var("GITHUB_ACTIONS"));
  });
}
