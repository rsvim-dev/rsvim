//! Messages that are sent to the javascript runtime (see [`get_state`](crate::js::get_state)).

use crate::buf::BufferId;
use crate::js::TaskId;
use crate::js::TimerId;
use crate::prelude::*;
use crate::ui::tree::NodeId;
use compact_str::CompactString;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::Instant;

#[derive(Debug)]
/// Message sent to the javascript runtime.
pub enum JsMessage {
  /// Event loop notify Js runtime to shutdown this thread.
  TimeoutResp(TimeoutResp),

  /// Event loop send ex command to js runtime to run.
  ExCommandReq(ExCommandReq),

  /// Master send js runtime the result of fs open
  FsOpenResp(FsOpenResp),

  /// Master send js runtime the result of fs read
  FsReadResp(FsReadResp),

  /// Master send js runtime the result of fs write
  FsWriteResp(FsWriteResp),

  /// Master send js runtime the result of fs read_file
  FsReadFileResp(FsReadFileResp),

  /// Master send js runtime the result of fs read_text_file
  FsReadTextFileResp(FsReadTextFileResp),

  /// Master send js runtime the result of fs read_dir.
  FsReadDirResp(FsReadDirResp),

  /// Master send js runtime the result of fs read_dir_next.
  FsReadDirNextResp(FsReadDirNextResp),

  /// Master send js runtime the result of fs status
  FsStatResp(FsStatResp),

  /// Master send js runtime the result of fs symlink.
  FsSymlinkResp(FsSymlinkResp),

  /// Master send js runtime the result of fs hard link.
  FsLinkResp(FsLinkResp),

  /// Master send js runtime the result of fs create directory.
  FsMkdirResp(FsMkdirResp),

  /// Master send js runtime the result of load tree-sitter parser.
  LoadTreeSitterParserResp(LoadTreeSitterParserResp),

  /// Master send js runtime the result of child process stdio read text.
  ReadTextFromChildProcessStdioResp(ReadTextFromChildProcessStdioResp),

  /// Master send js runtime the result of child process finish.
  WaitChildProcessResp(WaitChildProcessResp),
}

#[derive(Debug)]
pub struct TimeoutResp {
  pub timer_id: TimerId,
  pub expire_at: Instant,
  pub delay: u32,
  pub repeated: bool,
}

#[derive(Debug)]
pub struct ExCommandReq {
  pub payload: CompactString,
  pub current_buf_id: BufferId,
  pub current_win_id: NodeId,
}

#[derive(Debug)]
pub struct FsOpenResp {
  pub task_id: TaskId,

  // type: `usize`
  pub maybe_result: Option<TheResult<Vec<u8>>>,
}

#[derive(Debug)]
pub struct FsReadResp {
  pub task_id: TaskId,

  // type: `Vec<u8>`
  pub maybe_result: Option<TheResult<Vec<u8>>>,
}

#[derive(Debug)]
pub struct FsReadFileResp {
  pub task_id: TaskId,

  // type: `Vec<u8>`
  pub maybe_result: Option<TheResult<Vec<u8>>>,
}

#[derive(Debug)]
pub struct FsReadTextFileResp {
  pub task_id: TaskId,

  // type: `Vec<u8>`
  pub maybe_result: Option<TheResult<Vec<u8>>>,
}

#[derive(Debug)]
pub struct FsReadDirResp {
  pub task_id: TaskId,

  // type: `ResourceId`
  pub maybe_result: Option<TheResult<Vec<u8>>>,
}

#[derive(Debug)]
pub struct FsReadDirNextResp {
  pub task_id: TaskId,

  // type: `FsDirEntry`
  pub maybe_result: Option<TheResult<Vec<u8>>>,
}

#[derive(Debug)]
pub struct FsStatResp {
  pub task_id: TaskId,

  // type: `FsFileInfo`
  pub maybe_result: Option<TheResult<Vec<u8>>>,
}

#[derive(Debug)]
pub struct FsSymlinkResp {
  pub task_id: TaskId,

  // type: `u32`
  pub maybe_result: Option<TheResult<Vec<u8>>>,
}

#[derive(Debug)]
pub struct FsLinkResp {
  pub task_id: TaskId,

  // type: `u32`
  pub maybe_result: Option<TheResult<Vec<u8>>>,
}

#[derive(Debug)]
pub struct FsMkdirResp {
  pub task_id: TaskId,

  // type: `u32`
  pub maybe_result: Option<TheResult<Vec<u8>>>,
}

#[derive(Debug)]
pub struct FsWriteResp {
  pub task_id: TaskId,

  // type: `usize`
  pub maybe_result: Option<TheResult<Vec<u8>>>,
}

#[derive(Debug)]
pub struct LoadTreeSitterParserResp {
  pub task_id: TaskId,

  // type: `Vec<String>`
  pub maybe_result: Option<TheResult<Vec<u8>>>,
}

#[derive(Debug)]
pub struct ReadTextFromChildProcessStdioResp {
  pub task_id: TaskId,

  // type: `String`
  pub maybe_result: Option<TheResult<Vec<u8>>>,
}

#[derive(Debug)]
pub struct WaitChildProcessResp {
  pub task_id: TaskId,

  // type: `(/* success */ bool, /* exit code */ Option<i32>, /* signal */ Option<i32>)`
  pub maybe_result: Option<TheResult<Vec<u8>>>,
}

/// Send js message in sync/blocking way, with tokio's "current_runtime".
///
/// Unlike the old implementation this ignores a closed receiver: the only
/// way `send` fails is that the event loop is already gone, and aborting the
/// process with a panic turns a clean shutdown into a crash report.
pub fn send_to_jsrt(jsrt_tx: UnboundedSender<JsMessage>, message: JsMessage) {
  let _ = jsrt_tx.send(message);
}
