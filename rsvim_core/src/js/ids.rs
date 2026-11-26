//! Handles that name asynchronous work.
//!
//! The caller is handed an integer and the work is looked up by it in a table
//! when it finishes.

// Timer id, starts from 1.
#[derive(
  Copy, Clone, rsvim_macro::IncrementalId, serde::Serialize, serde::Deserialize,
)]
pub struct TimerId(#[start_from(1)] i32);

// Task id, starts from 1.
#[derive(
  Copy, Clone, rsvim_macro::IncrementalId, serde::Serialize, serde::Deserialize,
)]
pub struct TaskId(#[start_from(1)] usize);
