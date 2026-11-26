//! Js error.

use crate::evloop::Editor;
use crate::js::cmdline::send_cmdline_message;
use crate::prelude::*;
use rquickjs::Ctx;
use rquickjs::Exception;
use rquickjs::FromJs;
use rquickjs::Value;

/// Represents an uncaught javascript exception.
#[derive(Eq, PartialEq, Clone, Default)]
pub struct JsError {
  pub message: String,
  pub resource_name: String,
  pub line_number: Option<i64>,
  pub start_column: Option<i64>,
  pub stack: Option<String>,
}

impl JsError {
  pub fn message(&self) -> &String {
    &self.message
  }

  pub fn resource_name(&self) -> &String {
    &self.resource_name
  }

  pub fn line_number(&self) -> &Option<i64> {
    &self.line_number
  }

  pub fn start_column(&self) -> &Option<i64> {
    &self.start_column
  }

  pub fn stack(&self) -> &Option<String> {
    &self.stack
  }
}

impl JsError {
  /// Builds a [`JsError`] from a javascript value that is about to be reported.
  ///
  /// The value is usually an `Error` instance, but javascript allows throwing
  /// anything (`throw "boom"`, `Promise.reject(42)`), so anything that is not
  /// an `Error` is stringified instead of being treated as one.
  pub fn from_js_value<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> Self {
    if let Some(exception) = value
      .as_object()
      .and_then(|object| Exception::from_object(object.clone()))
    {
      return Self::from_exception(&exception);
    }

    let message = rquickjs::Coerced::<String>::from_js(ctx, value)
      .map(|c| c.0)
      .unwrap_or_else(|_| UNKNOWN_RESOURCE.into());
    Self::unlocated(message)
  }

  /// An error that says nothing about where it came from.
  pub fn unlocated(message: impl Into<String>) -> Self {
    JsError {
      message: message.into(),
      resource_name: UNKNOWN_RESOURCE.into(),
      ..Default::default()
    }
  }

  /// Builds a [`JsError`] from a [`rquickjs::Error`] with no pending exception.
  ///
  /// `rquickjs` distinguishes "the engine failed" from "javascript threw": the
  /// former has no `Error` object to inspect, only the error's own message. The
  /// caller checks [`Ctx::has_exception`] first and prefers
  /// [`JsError::from_js_value`] whenever there is a value to look at.
  pub fn from_js_error(error: rquickjs::Error) -> Self {
    Self::unlocated(error.to_string())
  }

  /// Builds a [`JsError`] from whatever a fallible `rquickjs` call returned.
  ///
  /// `Error::Exception` only says "javascript threw"; the exception itself is
  /// still pending on the context, and it has to be taken *here*. Left behind it
  /// would be mistaken for a new error by whatever runs next, and reported a
  /// second time under a generic message.
  pub fn from_error(ctx: &Ctx<'_>, error: rquickjs::Error) -> Self {
    if matches!(error, rquickjs::Error::Exception) {
      if let Some(err) = crate::js::runtime::capture_exception(ctx) {
        return err;
      }
    }
    Self::from_js_error(error)
  }

  /// Builds a [`JsError`] from an [`Exception`].
  ///
  /// This is the one place an `Error` object becomes a report, so every way an
  /// error can reach the user -- a thrown exception, a rejected promise, a
  /// throwing callback, `reportError` -- reads the same:
  ///
  /// - the message is `Name: message`, like `Error: 3` or
  ///   `ReferenceError: x is not defined` (QuickJS keeps the name apart);
  /// - the position is the first frame of the stack, which is where the error
  ///   was thrown. QuickJS has no separate message object carrying it.
  pub fn from_exception(exception: &Exception<'_>) -> Self {
    let name = exception
      .as_object()
      .get::<_, Option<String>>("name")
      .ok()
      .flatten()
      .filter(|name| !name.is_empty());
    let message = exception.message().unwrap_or_default();
    let message = match (name, message.is_empty()) {
      (Some(name), false) => format!("{name}: {message}"),
      (Some(name), true) => name,
      (None, _) => message,
    };

    let stack = exception.stack();
    let frame = stack.as_deref().and_then(first_stack_frame);
    JsError {
      message,
      resource_name: frame
        .as_ref()
        .map(|(file, _, _)| file.clone())
        .unwrap_or_else(|| UNKNOWN_RESOURCE.into()),
      line_number: frame.as_ref().map(|(_, line, _)| *line),
      start_column: frame.as_ref().map(|(_, _, column)| *column),
      stack,
    }
  }

  /// Whether the error says where it was thrown.
  pub fn has_location(&self) -> bool {
    self.line_number.is_some() && self.resource_name != UNKNOWN_RESOURCE
  }

  /// The one line the command line shows.
  ///
  /// `in_promise` marks a rejection nobody handled, which is reported by the
  /// host and not thrown at anyone: `Uncaught (in promise) Error: x (a.js:1:7)`.
  pub fn to_report(&self, in_promise: bool) -> String {
    let mut report = String::from("Uncaught ");
    if in_promise {
      report.push_str("(in promise) ");
    }
    report.push_str(&self.message);
    if self.has_location() {
      report.push_str(&format!(
        " ({}:{}:{})",
        self.resource_name,
        self.line_number.unwrap_or_default(),
        self.start_column.unwrap_or_default()
      ));
    }
    report
  }
}

/// What [`JsError::resource_name`] holds when the position is not known.
const UNKNOWN_RESOURCE: &str = "(unknown)";

/// The position of the first frame of a QuickJS stack trace.
///
/// A frame looks like `    at name (file.js:3:5)` or, for code without a name,
/// `    at file.js:3:5`. Native frames (`    at <native>`) have no position and
/// are skipped.
fn first_stack_frame(stack: &str) -> Option<(String, i64, i64)> {
  stack.lines().find_map(|line| {
    let frame = line.trim_start().strip_prefix("at ")?;
    let location = match frame.rfind('(') {
      Some(open) if frame.ends_with(')') => &frame[open + 1..frame.len() - 1],
      _ => frame,
    };
    // `file:line:column`, where the file may itself contain colons (`C:\a.js`).
    let mut parts = location.rsplitn(3, ':');
    let column = parts.next()?.parse().ok()?;
    let line = parts.next()?.parse().ok()?;
    let file = parts.next()?.to_string();
    Some((file, line, column))
  })
}

impl std::error::Error for JsError {}

impl std::fmt::Display for JsError {
  /// Displays a minified version of the error.
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(&self.to_report(false))
  }
}

impl std::fmt::Debug for JsError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "Uncaught {}", self.message)
  }
}

/// Reports an error to the command line.
///
/// Takes anything that is an [`TheErr`], a [`JsError`] included.
pub fn report_js_error(editor: &Editor, e: impl Into<TheErr>) {
  send_cmdline_message(editor, e.into().to_string());
}
