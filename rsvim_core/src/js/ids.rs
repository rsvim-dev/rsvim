//! Handles that name asynchronous work.
//!
//! Both engines hand the caller an integer and look it up in a map when the
//! work finishes, so the ids belong to neither engine.

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
