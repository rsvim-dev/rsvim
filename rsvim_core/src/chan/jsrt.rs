//! Messages that are sent to the javascript runtime, here call it "jsrt".

use crate::buf::BufferId;
use crate::js::ops::Completion;
use crate::ui::tree::NodeId;
use compact_str::CompactString;
use tokio::sync::mpsc::UnboundedSender;

/// Message sent to the javascript runtime.
///
/// Everything that can make javascript run *later* arrives here, in order, on
/// one queue: an ex command from the editor state machine, and the completion
/// of an asynchronous operation a script started. (Timers are the third source;
/// they live in a table the loop waits on, not in a channel.)
#[derive(Debug)]
pub enum JsMessage {
  /// Ask the js runtime to execute an ex command.
  ExCommandReq(ExCommandReq),

  /// An asynchronous operation finished. The payload is plain data; the loop
  /// turns it into a javascript value and settles the operation's promise.
  OpDone(Completion),
}

#[derive(Debug)]
pub struct ExCommandReq {
  pub payload: CompactString,
  pub current_buf_id: BufferId,
  pub current_win_id: NodeId,
}

/// Send jsrt message in sync/blocking way, with tokio's "current_runtime".
pub fn send_to_jsrt(jsrt_tx: UnboundedSender<JsMessage>, message: JsMessage) {
  let _ = jsrt_tx.send(message);
}
