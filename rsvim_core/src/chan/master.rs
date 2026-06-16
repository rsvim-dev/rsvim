//! Messages that are sent to [`EventLoop`](crate::evloop::EventLoop), here
//! call it "master".

use crate::buf::BufferId;
use crate::js::TaskId;
use crate::js::TimerId;
use crate::js::binding::global_rsvim::fs::mkdir::FsMkdirOptions;
use crate::js::binding::global_rsvim::fs::open::FsOpenOptions;
use crate::js::binding::global_rsvim::fs::symlink::FsSymlinkOptions;
use crate::js::resource::ResourceId;
use crate::prelude::*;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::Instant;

#[derive(Debug)]
/// Message sent to [`EventLoop`](crate::evloop::EventLoop).
pub enum MasterMessage {
  /// Js runtime ask master to set timeout, i.e. sleep
  TimeoutReq(TimeoutReq),

  /// Js runtime ask master to exit
  ExitReq(ExitReq),

  /// Js runtime ask master to load import
  LoadImportReq(LoadImportReq),

  /// Js runtime ask master to tick loop again.
  TickAgainReq,

  /// Js runtime ask master to open file.
  FsOpenReq(FsOpenReq),

  /// Js runtime ask master to read file with buffer.
  FsReadReq(FsReadReq),

  /// Js runtime ask master to write file.
  FsWriteReq(FsWriteReq),

  /// Js runtime ask master to read file into data bytes.
  FsReadFileReq(FsReadFileReq),

  /// Js runtime ask master to read text file into string.
  FsReadTextFileReq(FsReadTextFileReq),

  /// Js runtime ask master to get fs status.
  FsStatReq(FsStatReq),

  /// Js runtime ask master to create symlink.
  FsSymlinkReq(FsSymlinkReq),

  /// Js runtime ask master to create hard link.
  FsLinkReq(FsLinkReq),

  /// Js runtime ask master to make directory.
  FsMkdirReq(FsMkdirReq),

  /// Ask master to parse text for a syntax editing.
  SyntaxEditReq(SyntaxEditReq),

  /// Response master for syntax parsing complete.
  SyntaxEditResp(SyntaxEditResp),

  /// Js runtime ask master to load tree-sitter parser.
  LoadTreeSitterParserReq(LoadTreeSitterParserReq),

  /// Js runtime ask master to read from child process stdio as text.
  ReadTextFromChildProcessStdioReq(ReadTextFromChildProcessStdioReq),

  /// Js runtime ask master to wait child process finish.
  WaitChildProcessReq(WaitChildProcessReq),
}

#[derive(Debug)]
pub struct ExitReq {
  pub exit_code: i32,
}

#[derive(Debug)]
pub struct TimeoutReq {
  pub timer_id: TimerId,
  pub start_at: Instant,
  pub delay: u32,
  pub repeated: bool,
}

#[derive(Debug)]
pub struct LoadImportReq {
  pub task_id: TaskId,
  pub specifier: String,
}

#[derive(Debug)]
pub struct FsOpenReq {
  pub task_id: TaskId,
  pub path: PathBuf,
  pub options: FsOpenOptions,
}

#[derive(Debug)]
pub struct FsReadReq {
  pub task_id: TaskId,
  pub file_rid: ResourceId,
  pub bufsize: usize,
}

#[derive(Debug)]
pub struct FsWriteReq {
  pub task_id: TaskId,
  pub file_rid: ResourceId,
  pub buf: Vec<u8>,
}

#[derive(Debug)]
pub struct FsReadFileReq {
  pub task_id: TaskId,
  pub path: PathBuf,
}

#[derive(Debug)]
pub struct FsReadTextFileReq {
  pub task_id: TaskId,
  pub path: PathBuf,
}

#[derive(Debug)]
pub struct FsStatReq {
  pub task_id: TaskId,
  pub follow_symlink: bool,
  pub path: PathBuf,
}

#[derive(Debug)]
pub struct FsSymlinkReq {
  pub task_id: TaskId,
  pub oldpath: PathBuf,
  pub newpath: PathBuf,
  pub options: FsSymlinkOptions,
}

#[derive(Debug)]
pub struct FsLinkReq {
  pub task_id: TaskId,
  pub oldpath: PathBuf,
  pub newpath: PathBuf,
}

#[derive(Debug)]
pub struct FsMkdirReq {
  pub task_id: TaskId,
  pub path: PathBuf,
  pub options: FsMkdirOptions,
}

#[derive(Debug)]
pub struct SyntaxEditReq {
  pub buffer_id: BufferId,
}

#[derive(Debug)]
pub struct SyntaxEditResp {
  pub buffer_id: BufferId,
}

#[derive(Debug)]
pub struct LoadTreeSitterParserReq {
  pub task_id: TaskId,
  pub grammar_path: PathBuf,
}

#[derive(Debug)]
pub struct ReadTextFromChildProcessStdioReq {
  pub task_id: TaskId,
  pub rid: ResourceId,
}

#[derive(Debug)]
pub struct WaitChildProcessReq {
  pub task_id: TaskId,
  pub rid: ResourceId,
}

/// Send master message in sync/blocking way, with tokio's "current_runtime".
pub fn send_to_master(
  master_tx: UnboundedSender<MasterMessage>,
  message: MasterMessage,
) {
  master_tx.send(message).unwrap();
}
