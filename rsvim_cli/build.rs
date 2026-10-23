use rsvim_core::js::JsRuntimeForSnapshot;
use rsvim_core::js::v8_version;
use rsvim_core::prelude::*;

// pub const LOG: &str = "[RSVIM]";
pub const LOG: &str = "cargo:warning=[RSVIM]";

fn version() {
  let profile_env = std::env::var("PROFILE").unwrap_or("debug".to_string());
  let opt_level_env = std::env::var("OPT_LEVEL").unwrap_or("0".to_string());
  let debug_env = std::env::var("DEBUG").unwrap_or("0".to_string());
  let host = std::env::var("HOST").unwrap_or("unknown".to_string());
  println!(
    "{LOG} Env: profile={:?}, opt_level={:?}, debug={:?}, host={:?}",
    profile_env, opt_level_env, debug_env, host
  );

  let workspace_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
  let version = env!("CARGO_PKG_VERSION").to_string();

  // profile
  let is_release_profile = profile_env == "release"
    && (opt_level_env == "s" || opt_level_env == "z")
    && debug_env != "true";
  let profile = if is_release_profile {
    "release".to_string()
  } else if profile_env == "release" {
    "nightly".to_string()
  } else {
    profile_env.clone()
  };

  // git commit
  let git_commit = match gix::open(&workspace_dir) {
    Ok(repo) => {
      let head_id = repo.head_id().unwrap();
      let id = head_id.shorten_or_id();
      let id = id.to_string();
      Some(id)
    }
    Err(e) => {
      println!("{LOG} Failed to fetch git commit:{:?}", e);
      None
    }
  };

  let v8_version = v8_version();

  println!(
    "{LOG} Resolved: profile={:?}, host={:?}, version={:?}, git_commit={:?}, v8={:?}",
    profile, host, version, git_commit, v8_version
  );

  let mut resolved = format!(
    "version={}\nprofile={}\nhost={}\nv8={}\n",
    version, profile, host, v8_version
  );
  if let Some(git_commit) = git_commit {
    resolved = format!("{}git_commit={}\n", resolved, git_commit);
  }

  let out_dir = std::env::var("OUT_DIR").unwrap();
  let output_path = Path::new(&out_dir).join("RSVIM_VERSION.TXT");
  println!("{LOG} Writing version into {:?}...", output_path.as_path());

  std::fs::write(output_path.as_path(), resolved.as_bytes()).unwrap();
}

fn snapshot() {
  let js_runtime = JsRuntimeForSnapshot::new();
  let snapshot = js_runtime.create_snapshot();
  let snapshot = Box::from(&snapshot);
  let mut vec = Vec::with_capacity(snapshot.len());
  vec.extend_from_slice(&snapshot);

  let out_dir = std::env::var("OUT_DIR").unwrap();
  let output_path = Path::new(&out_dir).join("RSVIM_SNAPSHOT.BIN");
  println!("{LOG} Writing snapshot into {:?}...", output_path.as_path());
  std::fs::write(output_path.as_path(), vec.into_boxed_slice()).unwrap();
}

fn main() {
  version();
  snapshot();
}
