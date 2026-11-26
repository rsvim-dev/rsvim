//! Messages that are sent to [`Rsvim`](crate::evloop::Rsvim), here call it
//! "master".

use crate::buf::BufferId;
use tokio::sync::mpsc::UnboundedSender;

/// Message sent to [`Rsvim`](crate::evloop::Rsvim).
///
/// The javascript runtime needs very little from the event loop: asynchronous
/// work is a tokio task that reports back on the javascript channel (see
/// [`crate::js::ops`]), so filesystem work does not round-trip through this
/// one. What is left is what only the loop can do -- stop it, and ask the state
/// machine to parse a buffer.
#[derive(Debug)]
pub enum MasterMessage {
  /// Js runtime ask master to exit.
  ExitReq(ExitReq),

  /// Ask master to parse text for a syntax editing.
  SyntaxEditReq(SyntaxEditReq),

  /// Response master for syntax parsing complete.
  SyntaxEditResp(SyntaxEditResp),
}

#[derive(Debug)]
pub struct ExitReq {
  pub exit_code: i32,
}

#[derive(Debug)]
pub struct SyntaxEditReq {
  pub buffer_id: BufferId,
}

#[derive(Debug)]
pub struct SyntaxEditResp {
  pub buffer_id: BufferId,
}

/// Send master message in sync/blocking way, with tokio's "current_runtime".
pub fn send_to_master(
  master_tx: UnboundedSender<MasterMessage>,
  message: MasterMessage,
) {
  let _ = master_tx.send(message);
}
