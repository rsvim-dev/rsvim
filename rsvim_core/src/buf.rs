//! Vim buffers.

pub mod opt;
pub mod text;
pub mod undo;
pub mod unicode;

#[cfg(test)]
mod opt_tests;
#[cfg(test)]
mod text_tests;
#[cfg(test)]
mod undo_tests;
#[cfg(test)]
mod unicode_tests;

use crate::hl::ColorSchemeArc;
use crate::hl::ColorSchemeManagerWk;
use crate::prelude::*;
use crate::syntax::Syntax;
use crate::syntax::SyntaxManagerWk;
use compact_str::CompactString;
use compact_str::ToCompactString;
use opt::*;
use ropey::Rope;
use ropey::RopeBuilder;
use std::fs::Metadata;
use text::Text;
use tokio::time::Instant;
use undo::Undo;

// BufferId starts from 1.
#[derive(
  Copy, Clone, rsvim_macro::IncrementalId, serde::Serialize, serde::Deserialize,
)]
pub struct BufferId(#[start_from(1)] i32);

#[derive(Debug, rsvim_macro::ArcMutexPtr)]
/// The Vim buffer, it is the in-memory texts mapping to the filesystem.
///
/// It contains several internal data:
/// 1. File name that associated with filesystem.
/// 2. File contents.
/// 3. File metadata.
///
/// To stable and avoid data racing issues, all file IO operations are made in pure blocking and
/// single-threading manner. And buffer also provide a set of APIs that serves as middle-level
/// primitives which can be used to implement high-level Vim ex commands, etc.
pub struct Buffer {
  id: BufferId,
  text: Text,

  // file
  filename: Option<PathBuf>,
  file_extension: Option<CompactString>,
  absolute_filename: Option<PathBuf>,
  metadata: Option<Metadata>,
  last_sync_time: Option<Instant>,

  // undo manager
  undo: Undo,

  // syntax parser
  syntax: Option<Syntax>,

  // colorscheme/highlight
  colorscheme: Option<ColorSchemeArc>,

  // Text editing version
  editing_version: isize,
}

impl Buffer {
  /// NOTE: This API should not be used to create new buffer, please use
  /// [`BufferManager`] APIs to manage buffer instances.
  pub fn _new(
    opts: BufferOptions,
    canvas_size: U16Size,
    rope: Rope,
    filename: Option<PathBuf>,
    file_extension: Option<CompactString>,
    absolute_filename: Option<PathBuf>,
    metadata: Option<Metadata>,
    last_sync_time: Option<Instant>,
    syntax: Option<Syntax>,
    colorscheme: Option<ColorSchemeArc>,
  ) -> Self {
    let text = Text::new(opts, canvas_size, rope);
    Self {
      id: BufferId::next(),
      text,
      filename,
      file_extension,
      absolute_filename,
      metadata,
      last_sync_time,
      undo: Undo::new(100),
      syntax,
      colorscheme,
      editing_version: 0,
    }
  }

  pub fn id(&self) -> BufferId {
    self.id
  }

  pub fn text(&self) -> &Text {
    &self.text
  }

  pub fn text_mut(&mut self) -> &mut Text {
    &mut self.text
  }

  pub fn options(&self) -> &BufferOptions {
    self.text.options()
  }

  pub fn set_options(&mut self, options: &BufferOptions) {
    self.text.set_options(options);
  }

  pub fn filename(&self) -> &Option<PathBuf> {
    &self.filename
  }

  pub fn file_extension(&self) -> &Option<CompactString> {
    &self.file_extension
  }

  pub fn set_filename(&mut self, filename: Option<PathBuf>) {
    self.filename = filename;
  }

  pub fn absolute_filename(&self) -> &Option<PathBuf> {
    &self.absolute_filename
  }

  pub fn set_absolute_filename(&mut self, absolute_filename: Option<PathBuf>) {
    self.absolute_filename = absolute_filename;
  }

  pub fn metadata(&self) -> &Option<Metadata> {
    &self.metadata
  }

  pub fn set_metadata(&mut self, metadata: Option<Metadata>) {
    self.metadata = metadata;
  }

  pub fn last_sync_time(&self) -> &Option<Instant> {
    &self.last_sync_time
  }

  pub fn set_last_sync_time(&mut self, last_sync_time: Option<Instant>) {
    self.last_sync_time = last_sync_time;
  }

  pub fn undo(&self) -> &Undo {
    &self.undo
  }

  pub fn undo_mut(&mut self) -> &mut Undo {
    &mut self.undo
  }

  pub fn syntax(&self) -> &Option<Syntax> {
    &self.syntax
  }

  pub fn syntax_mut(&mut self) -> &mut Option<Syntax> {
    &mut self.syntax
  }

  pub fn set_syntax(&mut self, value: Option<Syntax>) {
    self.syntax = value;
  }

  pub fn colorscheme(&self) -> &Option<ColorSchemeArc> {
    &self.colorscheme
  }

  /// Text edit versioning
  pub fn editing_version(&self) -> isize {
    self.editing_version
  }

  pub fn increase_editing_version(&mut self) -> isize {
    self.editing_version = if self.editing_version == isize::MAX {
      0
    } else {
      self.editing_version + 1
    };

    self.editing_version
  }
}

impl Buffer {
  pub fn has_filename(&self) -> bool {
    debug_assert_eq!(self.filename.is_some(), self.absolute_filename.is_some());
    self.filename.is_some() && self.absolute_filename.is_some()
  }
}

#[derive(Debug, rsvim_macro::ArcMutexPtr)]
/// The manager for all normal (file) buffers.
///
/// NOTE: A buffer has its unique filepath (on filesystem), and there is at most 1 unnamed buffer.
pub struct BufferManager {
  // Buffers collection
  buffers: BTreeMap<BufferId, BufferArc>,

  // Buffers maps by absolute file path.
  buffers_by_path: FoldMap<Option<PathBuf>, BufferArc>,

  // Global-local options for buffers.
  global_local_options: BufferOptions,

  // Syntax manager
  syntax_manager: SyntaxManagerWk,

  // ColorScheme manager
  colorscheme_manager: ColorSchemeManagerWk,
}

impl BufferManager {
  pub fn new(
    syntax_manager: SyntaxManagerWk,
    colorscheme_manager: ColorSchemeManagerWk,
  ) -> Self {
    BufferManager {
      buffers: BTreeMap::new(),
      buffers_by_path: FoldMap::new(),
      global_local_options: BufferOptionsBuilder::default().build().unwrap(),
      syntax_manager,
      colorscheme_manager,
    }
  }

  /// Open a file with a newly created buffer.
  ///
  /// The file name must be unique and not existed, there are two use cases:
  /// 1. If the file exists on filesystem, the buffer will read the file contents into buffer.
  /// 2. If the file doesn't exist, the buffer will be empty but only set the file name.
  ///
  /// # Returns
  ///
  /// It returns the buffer ID if the buffer created successfully, also the reading operations must
  /// be successful if the file exists on filesystem.
  /// Otherwise it returns the error.
  ///
  /// # Panics
  ///
  /// If the file name already exists.
  ///
  /// NOTE: This is a primitive API.
  pub fn new_file_buffer(
    &mut self,
    canvas_size: U16Size,
    filename: &Path,
  ) -> TheResult<BufferId> {
    let abs_filename = match filename.absolutize() {
      Ok(abs_filename) => abs_filename.to_path_buf(),
      Err(e) => {
        trace!("Failed to absolutize filepath {:?}:{:?}", filename, e);
        return Err(TheErr::NormalizePathFailed(filename.to_path_buf(), e));
      }
    };

    debug_assert!(
      !self
        .buffers_by_path
        .contains_key(&Some(abs_filename.clone()))
    );

    let existed = match std::fs::exists(&abs_filename) {
      Ok(existed) => existed,
      Err(e) => {
        trace!("Failed to detect file {:?}:{:?}", filename, e);
        return Err(TheErr::FileNotFound(filename.to_path_buf(), e));
      }
    };

    let buf = if existed {
      self.read_file(canvas_size, filename, &abs_filename)?
    } else {
      let file_extension = filename
        .extension()
        .map(|e| e.to_string_lossy().to_compact_string());
      let syntax = self._make_syntax_by_file_ext(&file_extension)?;
      let colorscheme = self._get_colorscheme();
      Buffer::_new(
        *self.global_local_options(),
        canvas_size,
        Rope::new(),
        Some(filename.to_path_buf()),
        file_extension,
        Some(abs_filename.clone()),
        None,
        None,
        syntax,
        colorscheme,
      )
    };

    let buf_id = buf.id();
    let buf = Buffer::to_arc(buf);
    self.buffers.insert(buf_id, buf.clone());
    self.buffers_by_path.insert(Some(abs_filename), buf);
    Ok(buf_id)
  }

  /// Create new empty buffer without file name.
  ///
  /// The file name of this buffer is empty, i.e. the buffer is unnamed.
  ///
  /// # Returns
  ///
  /// It returns the buffer ID if there is no other unnamed buffers.
  ///
  /// # Panics
  ///
  /// If there is already other unnamed buffers.
  ///
  /// NOTE: This is a primitive API.
  pub fn new_empty_buffer(&mut self, canvas_size: U16Size) -> BufferId {
    debug_assert!(!self.buffers_by_path.contains_key(&None));

    let colorscheme = self._get_colorscheme();
    let buf = Buffer::_new(
      *self.global_local_options(),
      canvas_size,
      Rope::new(),
      None,
      None,
      None,
      None,
      None,
      None,
      colorscheme,
    );
    let buf_id = buf.id();
    let buf = Buffer::to_arc(buf);
    self.buffers.insert(buf_id, buf.clone());
    self.buffers_by_path.insert(None, buf);
    buf_id
  }

  /// Write (save) a buffer to filesystem.
  pub fn write_buffer(&self, buf_id: BufferId) -> TheResult<usize> {
    match self.buffers.get(&buf_id) {
      Some(buf) => {
        let mut buf = lock!(buf);
        if !buf.has_filename() {
          return Err(TheErr::BufferNoName(buf_id));
        }
        self.write_file(&mut buf)
      }
      None => Err(TheErr::BufferNotExist(buf_id)),
    }
  }

  #[cfg(test)]
  /// NOTE: This API should only be used for testing.
  pub fn _add_buffer(&mut self, buf: BufferArc) -> BufferId {
    let (buf_id, abs_filepath) = {
      let buf = lock!(buf);
      (buf.id(), buf.absolute_filename().clone())
    };
    self.buffers.insert(buf_id, buf.clone());
    if abs_filepath.is_none() {
      assert!(!self.buffers_by_path.contains_key(&None));
    }
    self.buffers_by_path.insert(abs_filepath, buf);
    buf_id
  }

  /// NOTE: This API should be never used by external logic, it is been exposed
  /// as `pub` API just for testing purpose.
  pub fn _make_syntax_by_file_ext(
    &self,
    file_extension: &Option<CompactString>,
  ) -> TheResult<Option<Syntax>> {
    let syntax_manager = self.syntax_manager.upgrade().unwrap();
    let syntax_manager = lock!(syntax_manager);
    syntax_manager.make_syntax_by_ext(file_extension)
  }

  fn _get_colorscheme(&self) -> Option<ColorSchemeArc> {
    let colorscheme_manager = self.colorscheme_manager.upgrade().unwrap();
    let colorscheme_manager = lock!(colorscheme_manager);
    colorscheme_manager.colorscheme()
  }
}

// Primitive APIs {
impl BufferManager {
  fn read_file(
    &self,
    canvas_size: U16Size,
    filename: &Path,
    abs_filename: &Path,
  ) -> TheResult<Buffer> {
    match std::fs::metadata(filename) {
      Ok(metadata) => match std::fs::read(filename) {
        Ok(data) => {
          let mut rope_builder = RopeBuilder::new();
          let fencoding = self.global_local_options().file_encoding();
          let payload = match fencoding {
            FileEncodingOption::Utf8 => String::from_utf8_lossy(&data),
          };
          rope_builder.append(&payload);
          let rope = rope_builder.finish();
          trace!("Read {} bytes from file {:?}", data.len(), filename);

          let file_extension = filename
            .extension()
            .map(|e| e.to_string_lossy().to_compact_string());
          let syntax = self._make_syntax_by_file_ext(&file_extension)?;
          let colorscheme = self._get_colorscheme();

          Ok(Buffer::_new(
            *self.global_local_options(),
            canvas_size,
            rope,
            Some(filename.to_path_buf()),
            file_extension,
            Some(abs_filename.to_path_buf()),
            Some(metadata),
            Some(Instant::now()),
            syntax,
            colorscheme,
          ))
        }
        Err(e) => Err(TheErr::OpenFileFailed(filename.to_path_buf(), e)),
      },
      Err(e) => Err(TheErr::OpenFileFailed(filename.to_path_buf(), e)),
    }
  }

  fn write_file(&self, buf: &mut Buffer) -> TheResult<usize> {
    let buf_id = buf.id();
    let filename = buf
      .filename()
      .as_ref()
      .unwrap()
      .to_string_lossy()
      .to_compact_string();
    let abs_filename = buf.absolute_filename().as_ref().unwrap();

    let payload = buf.text().rope().to_string();
    let data = payload.as_bytes();
    match std::fs::write(abs_filename, data) {
      Ok(_) => {
        match std::fs::metadata(abs_filename) {
          Ok(metadata) => buf.set_metadata(Some(metadata)),
          Err(e) => {
            error!(
              "Failed to fetch metadata from file {:?}: {:?}",
              abs_filename, e
            );
          }
        }
        buf.set_last_sync_time(Some(Instant::now()));
        Ok(data.len())
      }
      Err(e) => Err(TheErr::SaveBufferFailed(buf_id, filename, e)),
    }
  }
}
// Primitive APIs }

pub type BufferManagerKeys<'a> =
  std::collections::btree_map::Keys<'a, BufferId, BufferArc>;
pub type BufferManagerValues<'a> =
  std::collections::btree_map::Values<'a, BufferId, BufferArc>;
pub type BufferManagerIter<'a> =
  std::collections::btree_map::Iter<'a, BufferId, BufferArc>;

// Buffers {
impl BufferManager {
  pub fn is_empty(&self) -> bool {
    self.buffers.is_empty()
  }

  pub fn len(&self) -> usize {
    self.buffers.len()
  }

  pub fn remove(&mut self, id: &BufferId) -> Option<BufferArc> {
    self.buffers.remove(id)
  }

  pub fn get(&self, id: &BufferId) -> Option<&BufferArc> {
    self.buffers.get(id)
  }

  pub fn contains_key(&self, id: &BufferId) -> bool {
    self.buffers.contains_key(id)
  }

  pub fn keys(&self) -> BufferManagerKeys<'_> {
    self.buffers.keys()
  }

  pub fn values(&self) -> BufferManagerValues<'_> {
    self.buffers.values()
  }

  pub fn iter(&self) -> BufferManagerIter<'_> {
    self.buffers.iter()
  }

  pub fn first_key_value(&self) -> Option<(&BufferId, &BufferArc)> {
    self.buffers.first_key_value()
  }

  pub fn last_key_value(&self) -> Option<(&BufferId, &BufferArc)> {
    self.buffers.last_key_value()
  }
}
// Buffers }

// Options {
impl BufferManager {
  pub fn global_local_options(&self) -> &BufferOptions {
    &self.global_local_options
  }

  pub fn global_local_options_mut(&mut self) -> &mut BufferOptions {
    &mut self.global_local_options
  }

  pub fn set_global_local_options(&mut self, options: &BufferOptions) {
    self.global_local_options = *options;
  }
}
// Options }
