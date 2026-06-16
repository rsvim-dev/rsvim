//! Pending async tasks.

use crate::chan;
use crate::chan::MasterMessage;
use crate::js::JsRuntimeState;
use crate::js::TaskId;
use crate::js::TimerId;
use crate::js::binding::global_rsvim::fs::open::FsOpenOptions;
use crate::js::binding::global_rsvim::fs::symlink::FsSymlinkOptions;
use crate::js::resource::ResourceId;
use crate::prelude::*;
use tokio::time::Instant;

pub type TimerCallback = Box<dyn FnMut() + 'static>;
pub type TaskCallback = Box<dyn FnMut(Option<TheResult<Vec<u8>>>) + 'static>;

pub fn create_timer(
  state: &mut JsRuntimeState,
  timer_id: TimerId,
  delay: u32,
  repeated: bool,
  cb: TimerCallback,
) {
  state.pending_timers.insert(timer_id, cb);
  let start_at = Instant::now();
  chan::send_to_master(
    state.master_tx.clone(),
    MasterMessage::TimeoutReq(chan::TimeoutReq {
      timer_id,
      start_at,
      delay,
      repeated,
    }),
  );
}

pub fn remove_timer(
  state: &mut JsRuntimeState,
  timer_id: TimerId,
) -> Option<TimerId> {
  state.pending_timers.remove(&timer_id).map(|_| timer_id)
}

pub fn create_import_loader(
  state: &mut JsRuntimeState,
  task_id: TaskId,
  specifier: &str,
  cb: TaskCallback,
) {
  state.pending_import_loaders.insert(task_id, cb);
  chan::send_to_master(
    state.master_tx.clone(),
    MasterMessage::LoadImportReq(chan::LoadImportReq {
      task_id,
      specifier: specifier.to_string(),
    }),
  );
}

pub fn create_fs_open(
  state: &mut JsRuntimeState,
  task_id: TaskId,
  path: &Path,
  options: FsOpenOptions,
  cb: TaskCallback,
) {
  state.pending_tasks.insert(task_id, cb);
  let path = path.to_path_buf();
  chan::send_to_master(
    state.master_tx.clone(),
    MasterMessage::FsOpenReq(chan::FsOpenReq {
      task_id,
      path,
      options,
    }),
  );
}

pub fn create_fs_read(
  state: &mut JsRuntimeState,
  task_id: TaskId,
  file_rid: ResourceId,
  bufsize: usize,
  cb: TaskCallback,
) {
  state.pending_tasks.insert(task_id, cb);
  chan::send_to_master(
    state.master_tx.clone(),
    MasterMessage::FsReadReq(chan::FsReadReq {
      task_id,
      file_rid,
      bufsize,
    }),
  );
}

pub fn create_fs_read_file(
  state: &mut JsRuntimeState,
  task_id: TaskId,
  path: &Path,
  cb: TaskCallback,
) {
  state.pending_tasks.insert(task_id, cb);
  chan::send_to_master(
    state.master_tx.clone(),
    MasterMessage::FsReadFileReq(chan::FsReadFileReq {
      task_id,
      path: path.to_path_buf(),
    }),
  );
}

pub fn create_fs_read_text_file(
  state: &mut JsRuntimeState,
  task_id: TaskId,
  path: &Path,
  cb: TaskCallback,
) {
  state.pending_tasks.insert(task_id, cb);
  chan::send_to_master(
    state.master_tx.clone(),
    MasterMessage::FsReadTextFileReq(chan::FsReadTextFileReq {
      task_id,
      path: path.to_path_buf(),
    }),
  );
}

pub fn create_fs_write(
  state: &mut JsRuntimeState,
  task_id: TaskId,
  file_rid: ResourceId,
  buf: Vec<u8>,
  cb: TaskCallback,
) {
  state.pending_tasks.insert(task_id, cb);
  chan::send_to_master(
    state.master_tx.clone(),
    MasterMessage::FsWriteReq(chan::FsWriteReq {
      task_id,
      file_rid,
      buf,
    }),
  );
}

pub fn create_fs_stat(
  state: &mut JsRuntimeState,
  task_id: TaskId,
  follow_symlink: bool,
  path: &Path,
  cb: TaskCallback,
) {
  state.pending_tasks.insert(task_id, cb);
  chan::send_to_master(
    state.master_tx.clone(),
    MasterMessage::FsStatReq(chan::FsStatReq {
      task_id,
      follow_symlink,
      path: path.to_path_buf(),
    }),
  );
}

pub fn create_fs_symlink(
  state: &mut JsRuntimeState,
  task_id: TaskId,
  oldpath: &Path,
  newpath: &Path,
  options: FsSymlinkOptions,
  cb: TaskCallback,
) {
  state.pending_tasks.insert(task_id, cb);
  chan::send_to_master(
    state.master_tx.clone(),
    MasterMessage::FsSymlinkReq(chan::FsSymlinkReq {
      task_id,
      oldpath: oldpath.to_path_buf(),
      newpath: newpath.to_path_buf(),
      options,
    }),
  );
}

pub fn create_fs_link(
  state: &mut JsRuntimeState,
  task_id: TaskId,
  oldpath: &Path,
  newpath: &Path,
  cb: TaskCallback,
) {
  state.pending_tasks.insert(task_id, cb);
  chan::send_to_master(
    state.master_tx.clone(),
    MasterMessage::FsLinkReq(chan::FsLinkReq {
      task_id,
      oldpath: oldpath.to_path_buf(),
      newpath: newpath.to_path_buf(),
    }),
  );
}

pub fn create_syn_load_parser(
  state: &mut JsRuntimeState,
  task_id: TaskId,
  grammar_path: &Path,
  cb: TaskCallback,
) {
  state.pending_tasks.insert(task_id, cb);
  chan::send_to_master(
    state.master_tx.clone(),
    MasterMessage::LoadTreeSitterParserReq(chan::LoadTreeSitterParserReq {
      task_id,
      grammar_path: grammar_path.to_path_buf(),
    }),
  );
}

pub fn create_read_text_from_child_process_stdio(
  state: &mut JsRuntimeState,
  task_id: TaskId,
  rid: ResourceId,
  cb: TaskCallback,
) {
  state.pending_tasks.insert(task_id, cb);
  chan::send_to_master(
    state.master_tx.clone(),
    MasterMessage::ReadTextFromChildProcessStdioReq(
      chan::ReadTextFromChildProcessStdioReq { task_id, rid },
    ),
  );
}

pub fn create_wait_child_process(
  state: &mut JsRuntimeState,
  task_id: TaskId,
  rid: ResourceId,
  cb: TaskCallback,
) {
  state.pending_tasks.insert(task_id, cb);
  chan::send_to_master(
    state.master_tx.clone(),
    MasterMessage::WaitChildProcessReq(chan::WaitChildProcessReq {
      task_id,
      rid,
    }),
  );
}
