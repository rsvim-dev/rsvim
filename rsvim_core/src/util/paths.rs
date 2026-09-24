//! File path utils.

use std::borrow::Cow;
use std::ffi::OsStr;
use std::io::Result;
use std::path::Path;
use std::path::PathBuf;

/// Map path to its parent, or remain unchanged if it has no parent.
pub fn maybe_parent<S: AsRef<OsStr> + ?Sized>(s: &S) -> &Path {
  Path::new(s).parent().unwrap_or(Path::new(s))
}

pub trait PathExt {
  /// Normalize file path and resolve symbolic link.
  fn normalize(&self) -> Result<PathBuf>;

  // /// Normalize file path, without resolving symbolic link.
  // fn normalize_without_symbolic(&self) -> Cow<'_, Path>;

  /// Absolutize file path, relative to current directory.
  fn absolutize(&self) -> Result<Cow<'_, Path>>;

  /// Same with [`PathExt::absolutize`], but relative to parameter `cwd` as current directory.
  fn absolutize_with(&self, cwd: &Path) -> Result<Cow<'_, Path>>;
}

impl PathExt for Path {
  fn normalize(&self) -> Result<PathBuf> {
    normpath::PathExt::normalize(self).map(|p| p.into_path_buf())
  }

  // fn normalize_without_symbolic(&self) -> Cow<'_, Path> {
  //   sugar_path::SugarPath::normalize(self)
  // }

  fn absolutize(&self) -> Result<Cow<'_, Path>> {
    path_absolutize::Absolutize::absolutize(self)
  }

  fn absolutize_with(&self, cwd: &Path) -> Result<Cow<'_, Path>> {
    Ok(path_absolutize::Absolutize::absolutize_from(self, cwd))
  }
}
