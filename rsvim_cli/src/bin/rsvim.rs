//! The VIM editor reinvented in Rust+TypeScript.
//!
//! See [rsvim_core] for more details.

#[cfg(all(feature = "mimalloc", not(any(feature = "jemalloc"))))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(all(
  feature = "jemalloc",
  not(any(target_env = "msvc", feature = "mimalloc"))
))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

use clap::Parser;
use rsvim_core::cli::CliOptions;
use rsvim_core::evloop::EventLoop;
use rsvim_core::log;
use rsvim_core::prelude::*;
use std::sync::LazyLock;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use tokio::time::Instant;

static RSVIM_VERSION: LazyLock<String> = LazyLock::new(|| {
  let version_tags =
    include_str!(concat!(env!("OUT_DIR"), "/RSVIM_VERSION.TXT"));

  let version_tags = version_tags
    .split("\n")
    .filter(|line| {
      !line.is_empty() && line.split("=").collect::<Vec<&str>>().len() == 2
    })
    .map(|line| {
      let key_value = line.split("=").collect::<Vec<&str>>();
      let key = key_value[0].trim();
      let value = key_value[1].trim();
      (key.to_string(), value.to_string())
    })
    .collect::<FoldMap<String, String>>();
  let git_commit = match version_tags.get("git_commit") {
    Some(git_commit) => format!("{}, ", git_commit),
    None => "".to_string(),
  };
  let binary_version = format!(
    "{} {} ({}{}, {})",
    env!("CARGO_BIN_NAME"),
    version_tags["version"],
    git_commit,
    version_tags["profile"],
    version_tags["host"]
  );
  let features = if cfg!(feature = "jemalloc") {
    "features: +jemalloc\n"
  } else if cfg!(feature = "mimalloc") {
    "features: +mimalloc\n"
  } else {
    ""
  };
  let v8_version = format!("v8 {}", version_tags["v8"]);
  format!("{}\n{}{}", binary_version, features, v8_version)
});

fn main() -> IoResult<()> {
  // Startup time
  let startup_moment = Instant::now();
  let startup_unix_epoch = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_millis();

  log::init();

  let cli_opts = CliOptions::parse();
  trace!("cli_opts:{:?}", cli_opts);

  if cli_opts.version() {
    println!("{}", *RSVIM_VERSION);
    std::process::exit(0);
  }

  // Explicitly create tokio runtime for the EventLoop.
  let rt = tokio::runtime::Runtime::new()?;
  rt.block_on(async {
    // Create event loop.
    let mut event_loop =
      EventLoop::new(startup_moment, startup_unix_epoch, cli_opts)?;

    // Initialize.
    event_loop.initialize()?;

    // Run loop.
    let exit_code = event_loop.run().await?;

    // Shutdown.
    event_loop.shutdown()?;

    std::process::exit(exit_code);
  })
}
