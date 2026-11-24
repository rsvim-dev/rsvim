//! The `/dev/null` mode writer.

use crate::evloop::writer::StdoutWritable;
use crate::prelude::*;
use crate::ui::canvas::Canvas;
use crate::ui::canvas::ShaderCommand;

#[derive(Debug)]
/// The `/dev/null` mode writer, it eats everything and print nothing.
pub struct DevNullWriter {
  shaders: Vec<ShaderCommand>,
  /// How many times [`StdoutWritable::write`] was actually called.
  ///
  /// Counting the calls (instead of the shader commands) is deliberate:
  /// [`Canvas::shade`] legitimately yields an empty command list when nothing
  /// on screen changed, so counting commands would conflate "did not repaint"
  /// with "repainted an identical frame".
  write_count: usize,
}

impl DevNullWriter {
  pub fn new() -> Self {
    Self {
      shaders: vec![],
      write_count: 0,
    }
  }
}

impl StdoutWritable for DevNullWriter {
  fn init(&self) -> IoResult<()> {
    Ok(())
  }

  fn init_complete(&mut self, _canvas: &mut Canvas) -> IoResult<()> {
    Ok(())
  }

  fn shutdown(&self) -> IoResult<()> {
    Ok(())
  }

  fn write(&mut self, canvas: &mut Canvas) -> IoResult<()> {
    self.write_count += 1;
    let shaders = canvas.shade();
    let shaders = lock!(shaders);
    self.shaders.extend_from_slice(&shaders);
    Ok(())
  }
}

impl DevNullWriter {
  pub fn shaders(&self) -> &Vec<ShaderCommand> {
    &self.shaders
  }

  /// How many times the writer was asked to repaint.
  pub fn write_count(&self) -> usize {
    self.write_count
  }
}
