//! Errors and results.

use crate::buf::BufferId;
use crate::js::err::JsError;
use crate::js::module::ModulePath;
use crate::js::resource::ResourceId;
use compact_str::CompactString;
use std::path::PathBuf;
use tree_sitter::LanguageError;
use tree_sitter_loader::LoaderError;

// std::io {

/// [`std::io::Error`]
pub type IoErr = std::io::Error;

/// [`std::io::ErrorKind`]
pub type IoErrKind = std::io::ErrorKind;

/// [`std::io::Result`] with `T` if ok.
pub type IoResult<T> = std::io::Result<T>;

// std::io }

// thiserror {

/// All error codes.
#[derive(Debug, thiserror::Error)]
pub enum TheErr {
  #[error("Buffer `{0}` doesn't have a filename.")]
  BufferNoName(BufferId),

  #[error("Buffer `{0}` not exist.")]
  BufferNotExist(BufferId),

  #[error("Failed to save buffer `{0}`({1}): {2}.")]
  SaveBufferFailed(BufferId, CompactString, IoErr),

  #[error("Failed to load syntax for language {0}: {1}.")]
  LoadSyntaxFailed(CompactString, LanguageError),

  #[error("Failed to load tree-sitter parser {0}: {1}.")]
  LoadTreeSitterParserFailed(CompactString, LoaderError),

  #[error("Tree-sitter parser not found: {0}.")]
  TreeSitterParserNotFound(CompactString),

  #[error("Failed to load colorscheme: {0}.")]
  LoadColorSchemeFailed(CompactString),

  #[error("ColorScheme `{0}` not found.")]
  ColorSchemeNotFound(CompactString),

  #[error("Undo commit `{0}` not exist.")]
  UndoCommitNotExist(usize),

  #[error("Failed to normalize path `{0}`: {1}.")]
  NormalizePathFailed(PathBuf, IoErr),

  #[error("Command `{0}` not found.")]
  CommandNotFound(CompactString),

  #[error("Command `{0}` already exist.")]
  CommandAlreadyExist(CompactString),

  #[error("{0}")]
  JsError(Box<JsError>),

  #[error("Failed to load module `{0}`: {1}.")]
  LoadModuleFailed(ModulePath, IoErr),

  #[error("Module `{0}` not found.")]
  ModuleNotFound(ModulePath),

  #[error("Failed to compile typescript: {0}.")]
  CompileTypeScriptFailed(CompactString),

  #[error("Not enough arguments specified.")]
  ArgumentsNotEnough,

  #[error("File `{0}` not found: {1}.")]
  FileNotFound(PathBuf, IoErr),

  #[error("Failed to open file `{0}`: {1}.")]
  OpenFileFailed(PathBuf, IoErr),

  #[error("Failed to read file `{0}`: {1}.")]
  ReadFileByRidFailed(ResourceId, IoErr),

  #[error("Failed to read file `{0}`: {1}.")]
  ReadFileByPathFailed(PathBuf, IoErr),

  #[error("Failed to write file `{0}`: {1}.")]
  WriteFileByRidFailed(ResourceId, IoErr),

  #[error("Failed to create symbolic link `{0}` pointing to `{1}`: {2}.")]
  CreateSymlinkFailed(PathBuf, PathBuf, IoErr),

  #[error("Failed to create hard link `{0}` pointing to `{1}`: {2}.")]
  CreateLinkFailed(PathBuf, PathBuf, IoErr),

  #[error("Failed to create directory `{0}`: {1}.")]
  CreateDirectoryFailed(PathBuf, IoErr),

  #[error("Invalid data.")]
  DataInvalid,

  #[error("Value too large: `{0}`.")]
  ValueTooLarge(usize),

  #[error("Buffer too small: `{0}`.")]
  BufferTooSmall(usize),

  #[error("Failed to spawn child process `{0}`: {1}.")]
  SpawnChildProcessFailed(CompactString, IoErr),

  #[error("Failed to read child process stdio `{0}`: {1}.")]
  ReadChildProcessStdioFailed(ResourceId, IoErr),

  #[error("Child process `{0}` not found.")]
  ChildProcessNotFound(ResourceId),

  #[error("Failed to wait child process `{0}`: {1}.")]
  WaitChildProcessFailed(ResourceId, IoErr),
}

/// [`Result`] with `T` if ok, [`TheErr`] if error.
pub type TheResult<T> = Result<T, TheErr>;

// thiserror }
