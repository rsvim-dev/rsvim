//! Child-process command.

use crate::js::converter::*;
use crate::js::resource::ResourceId;
use crate::js::resource::ResourceTableArc;
use crate::prelude::*;
use compact_str::CompactString;
use compact_str::ToCompactString;
use std::str::FromStr;

#[derive(
  Debug,
  Copy,
  Clone,
  PartialEq,
  Eq,
  PartialOrd,
  Ord,
  Hash,
  strum_macros::Display,
  strum_macros::EnumString,
)]
pub enum Stdio {
  #[strum(serialize = "null")]
  Null,

  #[strum(serialize = "piped")]
  Piped,

  #[strum(serialize = "inherit")]
  Inherit,
}

impl FromV8 for Stdio {
  fn from_v8<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    value: v8::Local<'s, v8::Value>,
  ) -> Self {
    debug_assert!(value.is_string() || value.is_string_object());
    let result = value.to_string(scope).unwrap().to_rust_string_lossy(scope);
    Stdio::from_str(&result).unwrap()
  }
}

impl ToV8 for Stdio {
  fn to_v8<'s>(
    &self,
    scope: &mut v8::PinScope<'s, '_>,
  ) -> v8::Local<'s, v8::Value> {
    self.to_string().to_v8(scope)
  }
}

#[derive(
  Debug,
  Clone,
  PartialEq,
  Eq,
  derive_builder::Builder,
  rsvim_macro::ToV8,
  rsvim_macro::FromV8,
)]
pub struct ProcessCommandOptions {
  #[builder(default = vec![])]
  pub args: Vec<CompactString>,

  #[builder(default = None)]
  pub cwd: Option<CompactString>,

  #[builder(default = false)]
  pub clear_env: bool,

  #[builder(default = false)]
  pub detached: bool,

  #[builder(default = FoldMap::new())]
  pub env: FoldMap<CompactString, CompactString>,

  #[builder(default = Stdio::Null)]
  pub stdin: Stdio,

  #[builder(default = Stdio::Piped)]
  pub stdout: Stdio,

  #[builder(default = Stdio::Piped)]
  pub stderr: Stdio,
}

pub fn spawn_child_process(
  resource_table: ResourceTableArc,
  exec_path: &str,
  options: &ProcessCommandOptions,
) -> TheResult<(
  /* child process */ ResourceId,
  /* child stdin */ Option<ResourceId>,
  /* child stdout */ Option<ResourceId>,
  /* child stderr */ Option<ResourceId>,
)> {
  let mut command = std::process::Command::new(exec_path);

  command.args(options.args.clone());
  if let Some(cwd) = &options.cwd {
    command.current_dir(cwd);
  }
  if options.clear_env {
    command.env_clear();
  }
  command.envs(
    options
      .env
      .clone()
      .into_iter()
      .map(|(k, v)| (k.into_string(), v)),
  );
  command.stdin(match options.stdin {
    Stdio::Null => std::process::Stdio::null(),
    Stdio::Piped => std::process::Stdio::piped(),
    Stdio::Inherit => std::process::Stdio::inherit(),
  });
  command.stdout(match options.stdout {
    Stdio::Null => std::process::Stdio::null(),
    Stdio::Piped => std::process::Stdio::piped(),
    Stdio::Inherit => std::process::Stdio::inherit(),
  });
  command.stderr(match options.stderr {
    Stdio::Null => std::process::Stdio::null(),
    Stdio::Piped => std::process::Stdio::piped(),
    Stdio::Inherit => std::process::Stdio::inherit(),
  });

  match command.spawn() {
    Ok(mut child) => {
      let stdin = child.stdin.take();
      let stdout = child.stdout.take();
      let stderr = child.stderr.take();
      let mut resource_table = lock!(resource_table);
      let stdin_rid = stdin.map(|s| resource_table.add_child_process_stdin(s));
      let stdout_rid =
        stdout.map(|s| resource_table.add_child_process_stdout(s));
      let stderr_rid =
        stderr.map(|s| resource_table.add_child_process_stderr(s));
      let child_rid = resource_table.add_child_process(child);
      Ok((child_rid, stdin_rid, stdout_rid, stderr_rid))
    }
    Err(e) => {
      let cmd = if !options.args.is_empty() {
        format!("{} {}", exec_path, options.args.join(" ")).to_compact_string()
      } else {
        exec_path.to_compact_string()
      };
      Err(TheErr::SpawnChildProcessFailed(cmd, e))
    }
  }
}
