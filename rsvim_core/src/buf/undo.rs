//! Undo history.

use crate::prelude::*;
use crate::util::ringbuf::DeRingBuffer;
use compact_str::CompactString;
use ropey::Rope;
use std::collections::VecDeque;
use std::fmt::Debug;
use tokio::time::Instant;

pub const INVALID_VERSION: usize = 0;
pub const START_VERSION: usize = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Insert {
  pub payload: CompactString,

  /// Absolute char idx of start insert position.
  pub start_char: usize,

  /// Absolute char idx of end insert position.
  pub end_char: usize,

  /// Cursor absolute char idx before insert.
  pub cursor_char_idx_before: usize,

  /// Cursor absolute char idx after insert.
  pub cursor_char_idx_after: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delete {
  pub payload: CompactString,

  /// Absolute char idx of start delete position.
  pub start_char: usize,

  /// Absolute char idx of end delete position.
  pub end_char: usize,

  /// Cursor absolute char idx before delete.
  pub cursor_char_idx_before: usize,

  /// Cursor absolute char idx after delete.
  pub cursor_char_idx_after: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// An operation is either a [`Insert`] or a [`Delete`].
/// The "Replace" operation can be converted into "Delete"+"Insert" operations.
///
/// Multiple operations can be merged into one operation. This can reduce
/// unnecessary operations inside one commit:
///
/// 1. Insert continuously chars `Hello, World`, actually we create 12
///    insertions: `H`, `e`, `l`, `l`, `o`, `,`, ` `, `W`, `o`, `r`, `l`, `d`.
///    We can merge these insertions into one `Hello, World`.
/// 2. First insert a char `a`, then delete it. Or first delete a char `b`,
///    then insert it back. Such kind of deletions can be deduplicated.
pub enum Operation {
  Insert(Insert),
  Delete(Delete),
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A record for operation with timestamp.
pub struct Record {
  pub op: Operation,
  pub moment: Instant,
  pub timestamp: jiff::Zoned,
  pub version: usize,
}

#[derive(Debug, Default, Clone)]
/// Undo manager maintains two parts:
/// 1. Uncommitted changes: When user starts insert mode, we will create a new
///    `Commit` struct to store all the uncommitted changes the user is going
///    to do.
/// 2. Committed history: When user finishes typing and switches to
///    other modes (normal, visual, etc) from insert mode, we will commit the
///    uncommitted changes to committed history.
///
/// NOTE:
/// 1. A operation record is a basic unit of undo/redo operation. Each time
///    user performs a undo/redo, the user operates on a operation record.
/// 2. For committed history, operation records will not be merged.
/// 3. For uncommitted changes, even they can be merged, there still can have
///    more than 1 operations. When we commit them, we will commit all of them
///    to undo manager.
pub struct Current {
  records: Vec<Record>,
}

impl Current {
  pub fn new() -> Self {
    Self { records: vec![] }
  }

  pub fn records(&self) -> &Vec<Record> {
    &self.records
  }

  pub fn records_mut(&mut self) -> &mut Vec<Record> {
    &mut self.records
  }

  pub fn delete(&mut self, op: Delete) {
    debug_assert!(op.start_char + op.payload.chars().count() == op.end_char);

    if op.payload.is_empty() {
      return;
    }

    if let Some(last_record) = self.records.last_mut()
      && let Operation::Delete(ref mut last) = last_record.op
      && op.start_char == last.end_char
    {
      // Merge 2 deletions
      trace!("last-1:{:?}, op:{:?}", last, op);
      last.payload.push_str(&op.payload);
      last.end_char = op.end_char;
      last_record.moment = Instant::now();
      last_record.timestamp = jiff::Zoned::now();
    } else if let Some(last_record) = self.records.last_mut()
      && let Operation::Delete(ref mut last) = last_record.op
      && op.end_char == last.start_char
    {
      // Merge 2 deletions
      trace!("last-2:{:?}, op:{:?}", last, op);
      last.payload.insert_str(0, &op.payload);
      last.start_char = op.start_char;
      last_record.moment = Instant::now();
      last_record.timestamp = jiff::Zoned::now();
    } else if let Some(last_record) = self.records.last_mut()
      && let Operation::Insert(ref mut last) = last_record.op
      && last.payload == op.payload
      && last.start_char == op.start_char
      && last.end_char == op.end_char
    {
      // Offset the effect of 1 insertion and 1 deletion
      trace!("last-3:{:?}, op:{:?}", last, op);
      self.records.pop();
    } else {
      trace!("last-4, op:{:?}", op);
      self.records.push(Record {
        op: Operation::Delete(op),
        moment: Instant::now(),
        timestamp: jiff::Zoned::now(),
        version: INVALID_VERSION,
      });
    }
  }

  pub fn insert(&mut self, op: Insert) {
    debug_assert_eq!(op.start_char + op.payload.chars().count(), op.end_char);

    if op.payload.is_empty() {
      return;
    }

    if let Some(last_record) = self.records.last_mut()
      && let Operation::Insert(ref mut last) = last_record.op
      && last.end_char == op.start_char
    {
      trace!("last-1:{:?}, op:{:?}", last, op);
      // Append to last insertion
      last.payload.push_str(&op.payload);
      last.end_char = op.end_char;
      last_record.moment = Instant::now();
      last_record.timestamp = jiff::Zoned::now();
    } else {
      trace!("last-2, op:{:?}", op);
      self.records.push(Record {
        op: Operation::Insert(op),
        moment: Instant::now(),
        timestamp: jiff::Zoned::now(),
        version: INVALID_VERSION,
      });
    }
  }
}

#[derive(Debug, Clone)]
pub struct Undo {
  undo_stack: DeRingBuffer<Record>,
  redo_stack: VecDeque<Record>,
  current: Current,
  __next_version: usize,
}

impl Undo {
  pub fn new(max_size: usize) -> Self {
    Self {
      undo_stack: DeRingBuffer::new(max_size),
      redo_stack: VecDeque::new(),
      current: Current::new(),
      __next_version: START_VERSION,
    }
  }

  fn next_version(&mut self) -> usize {
    let result = self.__next_version;
    self.__next_version += 1;
    result
  }

  pub fn current(&self) -> &Current {
    &self.current
  }

  pub fn current_mut(&mut self) -> &mut Current {
    &mut self.current
  }

  pub fn commit(&mut self) {
    let version = self.next_version();
    for mut change in self.current.records_mut().drain(..) {
      change.version = version;
      self.undo_stack.push_back_overwrite(change);
    }
    self.current = Current::new();
  }

  /// This is similar to `git revert` a specific git commit ID.
  /// It reverts to the previous `commit`.
  pub fn undo(&mut self, commit_idx: usize, rope: &mut Rope) -> TheResult<()> {
    if commit_idx >= self.undo_stack.len() {
      return Err(TheErr::UndoCommitNotExist(commit_idx));
    }

    let mut i: isize = commit_idx as isize;
    while i >= commit_idx as isize {
      let record = &self.undo_stack[i as usize];
      // Revert all editing operations on the passed `rope`.
      match &record.op {
        Operation::Insert(insert) => {
          trace!("rope.len_chars:{:?}, insert:{:?}", rope.len_chars(), insert);
          debug_assert!(rope.len_chars() >= insert.end_char);
          if cfg!(debug_assertions) {
            let range: std::ops::Range<usize> =
              insert.start_char..insert.end_char;
            let chars = rope.chars_at(range.start);
            debug_assert!(chars.len() >= insert.end_char - insert.start_char);
            let actual = chars
              .take(range.end - range.start)
              .collect::<CompactString>();
            debug_assert_eq!(actual, insert.payload);
          }
          rope.remove(insert.start_char..insert.end_char);
        }
        Operation::Delete(delete) => {
          trace!("rope.len_chars:{:?}, delete:{:?}", rope.len_chars(), delete);
          debug_assert!(rope.len_chars() >= delete.start_char);
          rope.insert(delete.start_char, &delete.payload);
        }
      }
      i -= 1;
    }

    for record in self.undo_stack.drain(commit_idx..).rev() {
      self.redo_stack.push_back(record);
    }

    Ok(())
  }

  pub fn undo_stack(&self) -> &DeRingBuffer<Record> {
    &self.undo_stack
  }

  pub fn redo_stack(&self) -> &VecDeque<Record> {
    &self.redo_stack
  }
}
