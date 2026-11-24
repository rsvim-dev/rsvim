//! Js error.

use crate::js::JsKernel;
use crate::js::binding::cmd::send_cmdline_message;
use crate::prelude::*;

/// Represents an exception coming from V8.
#[derive(Eq, PartialEq, Clone, Default)]
pub struct JsError {
  pub message: String,
  pub resource_name: String,
  pub source_line: Option<String>,
  pub line_number: Option<i64>,
  pub start_column: Option<i64>,
  pub end_column: Option<i64>,
  pub stack: Option<String>,
}

impl JsError {
  pub fn message(&self) -> &String {
    &self.message
  }

  pub fn resource_name(&self) -> &String {
    &self.resource_name
  }

  pub fn source_line(&self) -> &Option<String> {
    &self.source_line
  }

  pub fn line_number(&self) -> &Option<i64> {
    &self.line_number
  }

  pub fn start_column(&self) -> &Option<i64> {
    &self.start_column
  }

  pub fn end_column(&self) -> &Option<i64> {
    &self.end_column
  }

  pub fn stack(&self) -> &Option<String> {
    &self.stack
  }
}

impl JsError {
  // https://github.com/denoland/rusty_v8/blob/0d093a02f658781d52e6d70d138768fc19a79d54/examples/shell.rs#L158
  pub fn from_v8_exception<'s>(
    scope: &mut v8::PinScope,
    rejection: v8::Local<'s, v8::Value>,
    prefix: Option<&str>,
  ) -> Self {
    let message = v8::Exception::create_message(scope, rejection);

    let mut message_value = message
      .get(scope)
      .to_rust_string_lossy(scope)
      .replacen("Uncaught ", "", 1);

    // Check if message needs prefixing.
    if let Some(value) = prefix {
      message_value.insert_str(0, value);
    }

    let resource_name = message.get_script_resource_name(scope).map_or_else(
      || "(unknown)".into(),
      |s| s.to_string(scope).unwrap().to_rust_string_lossy(scope),
    );

    let source_line = message
      .get_source_line(scope)
      .map(|s| s.to_string(scope).unwrap().to_rust_string_lossy(scope));

    let line_number = message.get_line_number(scope).map(|num| num as i64);

    let start_column = Some(message.get_start_column() as i64);
    let end_column = Some(message.get_end_column() as i64);

    // Cast v8::PromiseRejectMessage to v8::Object so we can take it's `.stack` property.
    let exception = v8::Local::<v8::Object>::try_from(rejection);

    // Ignore source line when no stack-trace is available.
    let source_line = exception.map(|_| source_line).map(|s| s.unwrap()).ok();

    let stack = exception
      .map(|exception| {
        let stack = v8::String::new(scope, "stack").unwrap();
        let stack = exception.get(scope, stack.into());
        let stack: Option<v8::Local<v8::String>> =
          stack.and_then(|s| s.try_into().ok());
        stack.map(|s| s.to_rust_string_lossy(scope))
      })
      .map(|stack| stack.unwrap_or_default())
      .ok();

    JsError {
      message: message_value,
      resource_name,
      source_line,
      line_number,
      start_column,
      end_column,
      stack,
    }
  }
}

impl std::error::Error for JsError {}

impl std::fmt::Display for JsError {
  /// Displays a minified version of the error.
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    // Unwrap values.
    let line = self.line_number.unwrap_or_default();
    let column = self.start_column.unwrap_or_default();
    write!(
      f,
      "Uncaught {} ({}:{}:{})",
      self.message, self.resource_name, line, column
    )
  }
}

impl std::fmt::Debug for JsError {
  /// Displays a full version of the error with stack-trace.
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    // Output exception information.
    write!(f, "Uncaught {}", self.message)?;

    // Output source-line if exists.
    match self.source_line.as_ref() {
      Some(source_line) if !source_line.is_empty() => {
        // Log the source-line.
        writeln!(f, "\n{source_line}")?;

        // Indicate the position where the error was thrown.
        let start_column = self.start_column.unwrap_or_default();
        let end_column = self.end_column.unwrap_or_default();

        for _ in 0..start_column {
          write!(f, " ")?;
        }

        for _ in start_column..end_column {
          let mark = "^";
          write!(f, "{mark}")?;
        }

        // Print stacktrace if available.
        if let Some(stack) = self.stack.as_ref() {
          write!(f, "\n{stack}")?;
        }
      }
      _ => {}
    };

    Ok(())
  }
}

pub fn report_js_error(state: &JsKernel, e: TheErr) {
  send_cmdline_message(state, e.to_string());
}
