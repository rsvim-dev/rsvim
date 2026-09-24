//! Tree-sitter based syntax engine.

use crate::buf::Buffer;
use crate::prelude::*;
use compact_str::CompactString;
use compact_str::ToCompactString;
use itertools::Itertools;
use itertools::process_results;
use ropey::Rope;
use std::cmp::Ordering;
use std::fmt::Debug;
use std::ops::Range;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::Weak;
use tree_sitter::InputEdit;
use tree_sitter::Language;
use tree_sitter::LanguageError;
use tree_sitter::Parser;
use tree_sitter::Point;
use tree_sitter::Query;
use tree_sitter::QueryCursor;
use tree_sitter::StreamingIterator;
use tree_sitter::Tree;
use tree_sitter_loader::CompileConfig;
use tree_sitter_loader::Loader;

const INVALID_EDITING_VERSION: isize = -1;

pub type TreeSitterParserArc = Arc<Mutex<Parser>>;
pub type TreeSitterParserWk = Weak<Mutex<Parser>>;
pub type TreeSitterParserMutexGuard<'a> = MutexGuard<'a, Parser>;

#[derive(Clone, Debug)]
pub struct SyntaxEditNew {
  pub payload: Rope,
  pub version: isize,
}

#[derive(Clone, Debug)]
pub struct SyntaxEditUpdate {
  pub payload: Rope,
  pub input: InputEdit,
  pub version: isize,
}

#[derive(Debug, Clone)]
pub enum SyntaxEdit {
  New(SyntaxEditNew),
  Update(SyntaxEditUpdate),
}

pub type TreeSitterQueryArc = Arc<Query>;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct SyntaxCapturePoint {
  pub line_idx: usize,
  pub char_idx: usize,
}

impl Ord for SyntaxCapturePoint {
  fn cmp(&self, other: &Self) -> Ordering {
    match self.line_idx.cmp(&other.line_idx) {
      Ordering::Equal => self.char_idx.cmp(&other.char_idx),
      Ordering::Greater => Ordering::Greater,
      Ordering::Less => Ordering::Less,
    }
  }
}

impl PartialOrd for SyntaxCapturePoint {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
/// Convert [`tree_sitter::Range`] based bytes indexing into [`ropey::Rope`]
/// based chars indexing, i.e. use [`ropey::Rope::byte_to_char`] API to
/// transform byte index to char index.
pub struct SyntaxCaptureRange {
  pub start_char: usize,
  pub end_char: usize,
  pub start_point: SyntaxCapturePoint,
  pub end_point: SyntaxCapturePoint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxCaptureValue {
  pub index: u32,
  pub name: CompactString,
  pub range: SyntaxCaptureRange,
}

#[derive(Debug, Clone)]
pub struct SyntaxCaptureMultipleValues {
  pub values: Vec<SyntaxCaptureValue>,
  pub max_end_char: Option<usize>,
  pub max_end_point: Option<SyntaxCapturePoint>,
}

pub type SyntaxCaptureMap =
  FoldMap<SyntaxCapturePoint, SyntaxCaptureMultipleValues>;

#[derive(Debug, rsvim_macro::ArcPtr)]
pub struct SyntaxCapture {
  // Maps start_point to all its captured nodes.
  nodes: SyntaxCaptureMap,
}

impl SyntaxCapture {
  pub fn new(nodes: SyntaxCaptureMap) -> Self {
    Self { nodes }
  }

  pub fn nodes(&self) -> &SyntaxCaptureMap {
    &self.nodes
  }
}

// SyntaxId starts from 1.
#[derive(
  Copy, Clone, rsvim_macro::IncrementalId, serde::Serialize, serde::Deserialize,
)]
pub struct SyntaxId(#[start_from(1)] usize);

#[derive_where::derive_where(Debug)]
/// Buffer syntax.
pub struct Syntax {
  id: SyntaxId,

  // Highlight query
  highlight_query: Option<TreeSitterQueryArc>,
  highlight_capture: Option<SyntaxCaptureArc>,

  #[derive_where(skip)]
  // Parsed syntax tree
  tree: Option<Tree>,

  // Buffer's editing version of the syntax tree, this is copied from the
  // buffer's `editing_version` when starts parsing the buffer.
  editing_version: isize,

  #[derive_where(skip)]
  // Syntax parser
  parser: TreeSitterParserArc,

  // Filetype, i.e. programming language name, grammar id
  filetype: Option<CompactString>,

  // Pending edits that waiting for parsing
  pending_edits: Vec<SyntaxEdit>,

  // Whether the parser is already parsing the buffer text in a background
  // task. If true, it means the `parser` is parsing in a background task.
  //
  // NOTE: At a certain time, only 1 background task is parsing a buffer, there
  // will be no multiple background tasks parsing the same buffer
  // simultaneously, for data safety reason. New editings will be add to the
  // `pending` job queue and wait for the **current** running task complete,
  // then starts the next new task.
  parsing: bool,
}

impl Syntax {
  pub fn new(
    grammar: &Language,
    highlight_query: Option<&String>,
  ) -> Result<Self, LanguageError> {
    let filetype = grammar.name().map(|name| name.to_compact_string());
    let mut parser = Parser::new();
    parser.set_language(grammar)?;
    let parser = Arc::new(Mutex::new(parser));
    let highlight_query = match highlight_query {
      Some(source) => Query::new(grammar, source)
        .map(|q| Some(Arc::new(q)))
        .unwrap_or(None),
      None => None,
    };
    // trace!(
    //   "capture names:{:?}",
    //   highlight_query.as_ref().map(|hq| hq.capture_names())
    // );

    Ok(Self {
      id: SyntaxId::next(),
      highlight_query,
      highlight_capture: None,
      tree: None,
      editing_version: INVALID_EDITING_VERSION,
      parser,
      filetype,
      pending_edits: vec![],
      parsing: false,
    })
  }

  pub fn id(&self) -> SyntaxId {
    self.id
  }

  pub fn filetype(&self) -> &Option<CompactString> {
    &self.filetype
  }

  pub fn treesitter_highlight_query(&self) -> Option<TreeSitterQueryArc> {
    self.highlight_query.clone()
  }

  pub fn highlight_capture(&self) -> Option<SyntaxCaptureArc> {
    self.highlight_capture.clone()
  }

  pub fn set_highlight_capture(&mut self, value: Option<SyntaxCaptureArc>) {
    self.highlight_capture = value;
  }

  pub fn treesitter_tree(&self) -> &Option<Tree> {
    &self.tree
  }

  pub fn set_treesitter_tree(&mut self, tree: Option<Tree>) {
    self.tree = tree;
  }

  pub fn editing_version(&self) -> isize {
    self.editing_version
  }

  pub fn set_editing_version(&mut self, value: isize) {
    self.editing_version = value;
  }

  pub fn treesitter_parser(&self) -> TreeSitterParserArc {
    self.parser.clone()
  }

  pub fn is_parsing(&self) -> bool {
    self.parsing
  }

  pub fn set_is_parsing(&mut self, value: bool) {
    self.parsing = value;
  }

  pub fn pending_edits_is_empty(&self) -> bool {
    self.pending_edits.is_empty()
  }

  pub fn pending_edits_len(&self) -> usize {
    self.pending_edits.len()
  }

  pub fn add_pending_edits(&mut self, value: SyntaxEdit) {
    self.pending_edits.push(value);
  }

  pub fn drain_pending_edits<R>(
    &mut self,
    range: R,
  ) -> std::vec::Drain<'_, SyntaxEdit>
  where
    R: std::ops::RangeBounds<usize>,
  {
    self.pending_edits.drain(range)
  }
}

// pub type TreeSitterLoaderArc = Arc<Mutex<Loader>>;
// pub type TreeSitterLoaderWk = Weak<Mutex<Loader>>;
// pub type TreeSitterLoaderMutexGuard<'a> = MutexGuard<'a, Loader>;

#[derive_where::derive_where(Debug)]
#[derive(rsvim_macro::ArcPtr)]
pub struct SyntaxLoader {
  #[derive_where(skip)]
  loader: Mutex<Loader>,
}

#[derive(Debug, Clone)]
pub struct SyntaxLoadGrammarRequest {
  pub grammar_path: PathBuf,
}

impl SyntaxLoader {
  #[cfg(test)]
  pub fn new() -> Self {
    let loader = Loader::new().unwrap();
    Self {
      loader: Mutex::new(loader),
    }
  }

  #[cfg(not(test))]
  pub fn new() -> Self {
    use crate::cfg::path_cfg::PATH_CONFIG;

    let parser_lib_path =
      PATH_CONFIG.config_home().join(".tree-sitter-parsers");
    Self {
      loader: Mutex::new(Loader::with_parser_lib_path(parser_lib_path.clone())),
    }
  }

  pub fn treesitter_parser_lib_path(&self) -> PathBuf {
    lock!(self.loader).parser_lib_path.clone()
  }

  pub fn set_treesitter_parser_lib_path(&self, parser_lib_path: PathBuf) {
    lock!(self.loader).parser_lib_path = parser_lib_path;
  }
}

#[derive(Debug, Clone)]
pub struct SyntaxTreeSitterGrammarMetadata {
  pub name: CompactString,
  pub camelcase: CompactString,
  pub scope: CompactString,
  pub path: PathBuf,
  pub file_types: Vec<CompactString>,
  pub highlights_path: Option<PathBuf>,
  pub highlights_query: Option<String>,
  pub tags_path: Option<PathBuf>,
  pub tags_query: Option<String>,
  pub injections_path: Option<PathBuf>,
  pub injections_query: Option<String>,
  pub injection_regex: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SyntaxTreeSitterGrammarRepository {
  pub grammars: Vec<SyntaxTreeSitterGrammarMetadata>,

  /// The `tree-sitter-c`, `tree-sitter-rust` grammar directory path.
  pub grammar_path: PathBuf,

  /// The `src` directory path inside the grammar directory.
  pub src_path: PathBuf,

  /// The `grammar.json` file path inside `src` directory.
  pub grammar_json_path: PathBuf,
}

impl SyntaxLoader {
  fn parse_metadata(
    grammar_path: &Path,
    ts_json_grammars: &[serde_json::Value],
  ) -> TheResult<Vec<SyntaxTreeSitterGrammarMetadata>> {
    let err = || {
      TheErr::TreeSitterParserNotFound(
        grammar_path.to_string_lossy().to_compact_string(),
      )
    };

    let mut grammars = Vec::with_capacity(ts_json_grammars.len());
    for ts_grammar in ts_json_grammars {
      let name = ts_grammar.get("name").ok_or(err())?.as_str().ok_or(err())?;
      let camelcase = ts_grammar
        .get("camelcase")
        .ok_or(err())?
        .as_str()
        .ok_or(err())?;
      let scope = ts_grammar
        .get("scope")
        .ok_or(err())?
        .as_str()
        .ok_or(err())?;
      let path = ts_grammar.get("path").ok_or(err())?.as_str().ok_or(err())?;
      let path = grammar_path.join(path).canonicalize().map_err(|_e| err())?;
      let file_types = ts_grammar
        .get("file-types")
        .ok_or(err())?
        .as_array()
        .ok_or(err())?
        .iter()
        .map(|ft| ft.as_str().ok_or(err()));
      let file_types = process_results(file_types, |ft| ft.collect_vec())?
        .iter()
        .map(|ft| ft.to_compact_string())
        .collect_vec();
      let highlights_path = ts_grammar
        .get("highlights")
        .map(|hl| hl.as_str().ok_or(err()))
        .transpose()?
        .map(|hl| grammar_path.join(hl).normalize().map_err(|_e| err()))
        .transpose()?;
      let tags_path = ts_grammar
        .get("tags")
        .map(|tg| tg.as_str().ok_or(err()))
        .transpose()?
        .map(|tg| grammar_path.join(tg).normalize().map_err(|_e| err()))
        .transpose()?;
      let injections_path = ts_grammar
        .get("injections")
        .map(|inj| inj.as_str().ok_or(err()))
        .transpose()?
        .map(|inj| grammar_path.join(inj).normalize().map_err(|_e| err()))
        .transpose()?;
      let injection_regex = ts_grammar
        .get("injection-regex")
        .map(|tg| tg.as_str().ok_or(err()))
        .transpose()?
        .map(|inj| inj.to_string());
      let metadata = SyntaxTreeSitterGrammarMetadata {
        name: name.to_compact_string(),
        camelcase: camelcase.to_compact_string(),
        scope: scope.to_compact_string(),
        path,
        file_types,
        highlights_path,
        highlights_query: None,
        tags_path,
        tags_query: None,
        injections_path,
        injections_query: None,
        injection_regex,
      };
      grammars.push(metadata);
    }

    Ok(grammars)
  }

  fn parse_repository(
    grammar_path: &Path,
    ts_json_text: &str,
  ) -> TheResult<SyntaxTreeSitterGrammarRepository> {
    let err = || {
      TheErr::TreeSitterParserNotFound(
        grammar_path.to_string_lossy().to_compact_string(),
      )
    };
    let tree_sitter_json_data: serde_json::Value =
      serde_json::from_str(ts_json_text).map_err(|_e| err())?;

    let tree_sitter_json_grammars = tree_sitter_json_data
      .get("grammars")
      .ok_or(err())?
      .as_array()
      .ok_or(err())?;

    let grammars =
      Self::parse_metadata(grammar_path, tree_sitter_json_grammars)?;
    let src_path = grammar_path.join("src");
    let grammar_json_path = src_path.join("grammar.json");

    let repository = SyntaxTreeSitterGrammarRepository {
      grammars,
      grammar_path: grammar_path.to_path_buf(),
      src_path,
      grammar_json_path,
    };
    Ok(repository)
  }

  fn load_query(repository: &mut SyntaxTreeSitterGrammarRepository) {
    for grammar in repository.grammars.iter_mut() {
      grammar.highlights_query = match &grammar.highlights_path {
        Some(highlights_path) => std::fs::read_to_string(highlights_path).ok(),
        None => None,
      };
      grammar.tags_query = match &grammar.tags_path {
        Some(tags_path) => std::fs::read_to_string(tags_path).ok(),
        None => None,
      };
      grammar.injections_query = match &grammar.injections_path {
        Some(injections_path) => std::fs::read_to_string(injections_path).ok(),
        None => None,
      };
    }
  }

  async fn async_load_query(
    repository: &mut SyntaxTreeSitterGrammarRepository,
  ) {
    for grammar in repository.grammars.iter_mut() {
      grammar.highlights_query = match &grammar.highlights_path {
        Some(highlights_path) => {
          tokio::fs::read_to_string(highlights_path).await.ok()
        }
        None => None,
      };
      grammar.tags_query = match &grammar.tags_path {
        Some(tags_path) => tokio::fs::read_to_string(tags_path).await.ok(),
        None => None,
      };
      grammar.injections_query = match &grammar.injections_path {
        Some(injections_path) => {
          tokio::fs::read_to_string(injections_path).await.ok()
        }
        None => None,
      };
    }
  }

  fn parse_grammar_repository(
    grammar_path: &Path,
  ) -> TheResult<SyntaxTreeSitterGrammarRepository> {
    let err = || {
      TheErr::TreeSitterParserNotFound(
        grammar_path.to_string_lossy().to_compact_string(),
      )
    };

    let ts_json_path = grammar_path.join("tree-sitter.json");
    let ts_json_text =
      std::fs::read_to_string(ts_json_path).map_err(|_e| err())?;
    let mut repository = Self::parse_repository(grammar_path, &ts_json_text)?;
    Self::load_query(&mut repository);
    Ok(repository)
  }

  async fn async_parse_grammar_repository(
    grammar_path: &Path,
  ) -> TheResult<SyntaxTreeSitterGrammarRepository> {
    let err = || {
      TheErr::TreeSitterParserNotFound(
        grammar_path.to_string_lossy().to_compact_string(),
      )
    };

    let ts_json_path = grammar_path.join("tree-sitter.json");
    let ts_json_text = tokio::fs::read_to_string(ts_json_path)
      .await
      .map_err(|_e| err())?;
    let mut repository = Self::parse_repository(grammar_path, &ts_json_text)?;
    Self::async_load_query(&mut repository).await;
    Ok(repository)
  }

  /// Load the tree-sitter parser/grammar (`Language`) FFI dynamic library.
  pub fn load_grammar(
    &self,
    req: &SyntaxLoadGrammarRequest,
  ) -> TheResult<(
    /* metainfo */ SyntaxTreeSitterGrammarRepository,
    /* grammar */ Language,
  )> {
    let metainfo = Self::parse_grammar_repository(req.grammar_path.as_path())?;
    let compile_cfg =
      CompileConfig::new(metainfo.src_path.as_path(), None, None);
    match lock!(self.loader).load_language_at_path(compile_cfg) {
      Ok(grammar) => Ok((metainfo, grammar)),
      Err(e) => Err(TheErr::LoadTreeSitterParserFailed(
        req.grammar_path.to_string_lossy().to_compact_string(),
        e,
      )),
    }
  }

  pub async fn async_load_grammar(
    &self,
    req: &SyntaxLoadGrammarRequest,
  ) -> TheResult<(
    /* metainfo */ SyntaxTreeSitterGrammarRepository,
    /* grammar */ Language,
  )> {
    let metainfo =
      Self::async_parse_grammar_repository(req.grammar_path.as_path()).await?;
    let compile_cfg =
      CompileConfig::new(metainfo.src_path.as_path(), None, None);
    match lock!(self.loader).load_language_at_path(compile_cfg) {
      Ok(grammar) => Ok((metainfo, grammar)),
      Err(e) => Err(TheErr::LoadTreeSitterParserFailed(
        req.grammar_path.to_string_lossy().to_compact_string(),
        e,
      )),
    }
  }
}

pub fn load_syntax_grammar(
  syntax_manager: SyntaxManagerArc,
  req: &SyntaxLoadGrammarRequest,
) -> TheResult<SyntaxTreeSitterGrammarRepository> {
  let syntax_loader = lock!(syntax_manager).loader();
  let (metainfo, grammar) = syntax_loader.load_grammar(req)?;
  lock!(syntax_manager).insert_grammar(metainfo.clone(), grammar);
  Ok(metainfo)
}

pub async fn async_load_syntax_grammar(
  syntax_manager: SyntaxManagerArc,
  req: &SyntaxLoadGrammarRequest,
) -> TheResult<SyntaxTreeSitterGrammarRepository> {
  let syntax_loader = lock!(syntax_manager).loader();
  let (metainfo, grammar) = syntax_loader.async_load_grammar(req).await?;
  lock!(syntax_manager).insert_grammar(metainfo.clone(), grammar);
  Ok(metainfo)
}

#[derive(rsvim_macro::ArcMutexPtr)]
pub struct SyntaxManager {
  loader: SyntaxLoaderArc,

  // Loaded grammars (parsers)
  grammars: FoldMap<CompactString, Language>,

  // Loaded grammar metadata
  metadatas: FoldMap<CompactString, SyntaxTreeSitterGrammarMetadata>,

  // Maps file types to grammar names
  ftypes2names: FoldMap<CompactString, CompactString>,
}

impl Debug for SyntaxManager {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("SyntaxManager")
      .field("loader", &self.loader)
      .field("grammars", &self.grammars.keys())
      .field("ftypes2names", &self.ftypes2names)
      .finish()
  }
}

#[cfg(test)]
static TEST_GRAMMARS: std::sync::LazyLock<
  Vec<(SyntaxTreeSitterGrammarRepository, Language)>,
> = std::sync::LazyLock::new(|| {
  vec![
    (
      SyntaxTreeSitterGrammarRepository {
        grammars: vec![SyntaxTreeSitterGrammarMetadata {
          name: "c".to_compact_string(),
          camelcase: "C".to_compact_string(),
          scope: "".to_compact_string(),
          path: Path::new(".").to_path_buf(),
          file_types: vec!["c".to_compact_string(), "h".to_compact_string()],
          highlights_path: Some(
            Path::new("queries/highlights.scm").to_path_buf(),
          ),
          highlights_query: Some(tree_sitter_c::HIGHLIGHT_QUERY.to_string()),
          tags_path: Some(Path::new("queries/tags.scm").to_path_buf()),
          tags_query: Some(tree_sitter_c::TAGS_QUERY.to_string()),
          injections_path: None,
          injections_query: None,
          injection_regex: None,
        }],
        grammar_path: Path::new(".").to_path_buf(),
        src_path: Path::new(".").to_path_buf(),
        grammar_json_path: Path::new(".").to_path_buf(),
      },
      tree_sitter_c::LANGUAGE.into(),
    ),
    (
      SyntaxTreeSitterGrammarRepository {
        grammars: vec![SyntaxTreeSitterGrammarMetadata {
          name: "rust".to_compact_string(),
          camelcase: "Rust".to_compact_string(),
          scope: "".to_compact_string(),
          path: Path::new(".").to_path_buf(),
          file_types: vec!["rs".to_compact_string()],
          highlights_path: Some(
            Path::new("queries/highlights.scm").to_path_buf(),
          ),
          highlights_query: Some(
            tree_sitter_rust::HIGHLIGHTS_QUERY.to_string(),
          ),
          tags_path: Some(Path::new("queries/tags.scm").to_path_buf()),
          tags_query: Some(tree_sitter_rust::TAGS_QUERY.to_string()),
          injections_path: Some(
            Path::new("queries/injections.scm").to_path_buf(),
          ),
          injections_query: Some(
            tree_sitter_rust::INJECTIONS_QUERY.to_string(),
          ),
          injection_regex: None,
        }],
        grammar_path: Path::new(".").to_path_buf(),
        src_path: Path::new(".").to_path_buf(),
        grammar_json_path: Path::new(".").to_path_buf(),
      },
      tree_sitter_rust::LANGUAGE.into(),
    ),
    (
      SyntaxTreeSitterGrammarRepository {
        grammars: vec![SyntaxTreeSitterGrammarMetadata {
          name: "markdown".to_compact_string(),
          camelcase: "Markdown".to_compact_string(),
          scope: "".to_compact_string(),
          path: Path::new(".").to_path_buf(),
          file_types: vec![
            "md".to_compact_string(),
            "mdown".to_compact_string(),
          ],
          highlights_path: Some(
            Path::new("queries/highlights.scm").to_path_buf(),
          ),
          highlights_query: Some(
            tree_sitter_md::HIGHLIGHT_QUERY_BLOCK.to_string(),
          ),
          tags_path: None,
          tags_query: None,
          injections_path: Some(
            Path::new("queries/injections.scm").to_path_buf(),
          ),
          injections_query: Some(
            tree_sitter_md::INJECTION_QUERY_BLOCK.to_string(),
          ),
          injection_regex: None,
        }],
        grammar_path: Path::new(".").to_path_buf(),
        src_path: Path::new(".").to_path_buf(),
        grammar_json_path: Path::new(".").to_path_buf(),
      },
      tree_sitter_md::LANGUAGE.into(),
    ),
    (
      SyntaxTreeSitterGrammarRepository {
        grammars: vec![SyntaxTreeSitterGrammarMetadata {
          name: "toml".to_compact_string(),
          camelcase: "Toml".to_compact_string(),
          scope: "".to_compact_string(),
          path: Path::new(".").to_path_buf(),
          file_types: vec!["toml".to_compact_string()],
          highlights_path: Some(
            Path::new("queries/highlights.scm").to_path_buf(),
          ),
          highlights_query: Some(
            tree_sitter_toml_ng::HIGHLIGHTS_QUERY.to_string(),
          ),
          tags_path: None,
          tags_query: None,
          injections_path: None,
          injections_query: None,
          injection_regex: None,
        }],
        grammar_path: Path::new(".").to_path_buf(),
        src_path: Path::new(".").to_path_buf(),
        grammar_json_path: Path::new(".").to_path_buf(),
      },
      tree_sitter_toml_ng::LANGUAGE.into(),
    ),
    (
      SyntaxTreeSitterGrammarRepository {
        grammars: vec![SyntaxTreeSitterGrammarMetadata {
          name: "html".to_compact_string(),
          camelcase: "Html".to_compact_string(),
          scope: "".to_compact_string(),
          path: Path::new(".").to_path_buf(),
          file_types: vec![
            "htm".to_compact_string(),
            "html".to_compact_string(),
          ],
          highlights_path: Some(
            Path::new("queries/highlights.scm").to_path_buf(),
          ),
          highlights_query: Some(
            tree_sitter_html::HIGHLIGHTS_QUERY.to_string(),
          ),
          tags_path: None,
          tags_query: None,
          injections_path: None,
          injections_query: None,
          injection_regex: None,
        }],
        grammar_path: Path::new(".").to_path_buf(),
        src_path: Path::new(".").to_path_buf(),
        grammar_json_path: Path::new(".").to_path_buf(),
      },
      tree_sitter_html::LANGUAGE.into(),
    ),
  ]
});

// Language ID and file extensions {
impl SyntaxManager {
  fn _new() -> Self {
    Self {
      loader: SyntaxLoader::to_arc(SyntaxLoader::new()),
      grammars: FoldMap::new(),
      metadatas: FoldMap::new(),
      ftypes2names: FoldMap::new(),
    }
  }

  #[cfg(not(test))]
  pub fn new() -> Self {
    Self::_new()
  }

  #[cfg(test)]
  pub fn new() -> Self {
    let mut it = Self::_new();

    for grammar in TEST_GRAMMARS.iter() {
      it.insert_grammar(grammar.0.clone(), grammar.1.clone());
    }

    it
  }

  pub fn treesitter_parser_lib_path(&self) -> PathBuf {
    self.loader.treesitter_parser_lib_path()
  }

  /// NOTE: This will reset the tree-sitter loader and all loaded
  /// parsers/grammars.
  pub fn set_treesitter_parser_lib_path(&self, parser_lib_path: PathBuf) {
    self.loader.set_treesitter_parser_lib_path(parser_lib_path);
  }

  pub fn loader(&self) -> SyntaxLoaderArc {
    self.loader.clone()
  }
}
// Language ID and file extensions }

// Language and queries {
impl SyntaxManager {
  pub fn insert_grammar(
    &mut self,
    repository: SyntaxTreeSitterGrammarRepository,
    grammar: Language,
  ) {
    for metadata in repository.grammars.iter() {
      let name = &metadata.name;
      self.grammars.insert(name.clone(), grammar.clone());
      self.metadatas.insert(name.clone(), metadata.clone());
      for ftype in metadata.file_types.iter() {
        self.ftypes2names.insert(ftype.clone(), name.clone());
      }
    }
  }

  fn _internal_check(&self) {
    debug_assert_eq!(self.grammars.len(), self.metadatas.len());

    if cfg!(debug_assertions) {
      for name in self.grammars.keys() {
        debug_assert!(self.metadatas.contains_key(name));
      }
      for name in self.metadatas.keys() {
        debug_assert!(self.grammars.contains_key(name));
      }
      for (ftype, name) in self.ftypes2names.iter() {
        debug_assert!(self.metadatas.contains_key(name));
        debug_assert!(self.grammars.contains_key(name));
        debug_assert!(
          self
            .metadatas
            .get(name)
            .unwrap()
            .file_types
            .iter()
            .any(|ft| ft == ftype)
        );
      }
    }
  }

  pub fn list_grammar_names(&self) -> Vec<CompactString> {
    self._internal_check();
    self.grammars.keys().cloned().collect_vec()
  }

  pub fn get_grammar(&self, name: &str) -> Option<&Language> {
    self._internal_check();
    self.grammars.get(name)
  }

  pub fn get_metadata(
    &self,
    name: &str,
  ) -> Option<&SyntaxTreeSitterGrammarMetadata> {
    self._internal_check();
    self.metadatas.get(name)
  }

  pub fn get_highlights_query(&self, name: &str) -> Option<&String> {
    self._internal_check();
    match self.metadatas.get(name) {
      Some(mdata) => mdata.highlights_query.as_ref(),
      None => None,
    }
  }

  pub fn get_tags_query(&self, name: &str) -> Option<&String> {
    self._internal_check();
    match self.metadatas.get(name) {
      Some(mdata) => mdata.tags_query.as_ref(),
      None => None,
    }
  }

  pub fn get_injections_query(&self, name: &str) -> Option<&String> {
    self._internal_check();
    match self.metadatas.get(name) {
      Some(mdata) => mdata.injections_query.as_ref(),
      None => None,
    }
  }

  pub fn get_file_types_by_grammar_name(
    &self,
    name: &str,
  ) -> Option<&Vec<CompactString>> {
    self._internal_check();
    match self.metadatas.get(name) {
      Some(mdata) => Some(&mdata.file_types),
      None => None,
    }
  }

  pub fn get_grammar_name_by_file_type(
    &self,
    file_type: &str,
  ) -> Option<&CompactString> {
    self._internal_check();
    self.ftypes2names.get(file_type)
  }

  pub fn get_grammar_by_file_type(&self, file_type: &str) -> Option<&Language> {
    self._internal_check();
    self
      .ftypes2names
      .get(file_type)
      .map(|name| self.get_grammar(name))
      .unwrap_or(None)
  }

  /// Load/create a new Syntax by file extension.
  pub fn make_syntax_by_ext(
    &self,
    file_type: &Option<CompactString>,
  ) -> TheResult<Option<Syntax>> {
    if let Some(ext) = file_type
      && let Some(grammar) = self.get_grammar_by_file_type(ext)
    {
      trace!(
        "Load syntax by file type:{:?} grammar:{:?}",
        file_type,
        grammar.name()
      );
      let highlights_query = self
        .ftypes2names
        .get(ext)
        .map(|name| self.get_highlights_query(name))
        .unwrap_or(None);
      match Syntax::new(grammar, highlights_query) {
        Ok(syntax) => Ok(Some(syntax)),
        Err(e) => Err(TheErr::LoadSyntaxFailed(ext.clone(), e)),
      }
    } else {
      Ok(None)
    }
  }
}
// Language and queries }

fn convert_edit_char_to_byte(rope: &Rope, absolute_char_idx: usize) -> usize {
  rope
    .try_char_to_byte(absolute_char_idx)
    .unwrap_or(rope.len_bytes())
}

fn convert_edit_char_to_point(rope: &Rope, absolute_char_idx: usize) -> Point {
  if absolute_char_idx >= rope.len_chars() {
    let row = rope.len_lines();
    let column = 0;
    tree_sitter::Point { row, column }
  } else {
    let row = rope.char_to_line(absolute_char_idx);
    debug_assert!(rope.get_line(row).is_some());
    let relative_char_idx = absolute_char_idx - rope.line_to_char(row);
    let line = rope.line(row);
    let column = line
      .try_char_to_byte(relative_char_idx)
      .unwrap_or(line.len_bytes());
    tree_sitter::Point { row, column }
  }
}

pub fn make_input_edit_by_delete(
  buffer: &Buffer,
  absolute_char_idx_range: &Range<usize>,
) -> Option<InputEdit> {
  if buffer.syntax().is_some() {
    let start_byte = convert_edit_char_to_byte(
      buffer.text().rope(),
      absolute_char_idx_range.start,
    );
    let old_end_byte = convert_edit_char_to_byte(
      buffer.text().rope(),
      absolute_char_idx_range.end,
    );
    let new_end_byte = start_byte;
    let start_position = convert_edit_char_to_point(
      buffer.text().rope(),
      absolute_char_idx_range.start,
    );
    let old_end_position = convert_edit_char_to_point(
      buffer.text().rope(),
      absolute_char_idx_range.end,
    );
    let new_end_position = start_position;

    Some(InputEdit {
      start_byte,
      old_end_byte,
      new_end_byte,
      start_position,
      old_end_position,
      new_end_position,
    })
  } else {
    None
  }
}

pub fn make_input_edit_by_insert(
  buffer: &Buffer,
  absolute_char_idx: usize,
  absolute_end_char_idx: usize,
) -> Option<InputEdit> {
  if buffer.syntax().is_some() {
    let start_byte =
      convert_edit_char_to_byte(buffer.text().rope(), absolute_char_idx);
    let old_end_byte = start_byte;
    let new_end_byte =
      convert_edit_char_to_byte(buffer.text().rope(), absolute_end_char_idx);
    let start_position =
      convert_edit_char_to_point(buffer.text().rope(), absolute_char_idx);
    let old_end_position = start_position;
    let new_end_position =
      convert_edit_char_to_point(buffer.text().rope(), absolute_end_char_idx);
    Some(InputEdit {
      start_byte,
      old_end_byte,
      new_end_byte,
      start_position,
      old_end_position,
      new_end_position,
    })
  } else {
    None
  }
}

/// NOTE: Make this method public only for testing purpose.
pub fn _parse(
  parser: TreeSitterParserArc,
  old_tree: Option<Tree>,
  pending_edits: Vec<SyntaxEdit>,
) -> (Option<Tree>, isize, Option<Rope>, Option<String>) {
  let mut parser = lock!(parser);
  let mut tree = old_tree;
  let mut editing_version = INVALID_EDITING_VERSION;

  if cfg!(debug_assertions) {
    let mut new_count: usize = 0;
    for (i, edit) in pending_edits.iter().enumerate() {
      match edit {
        SyntaxEdit::New(_) => {
          debug_assert_eq!(i, 0);
          debug_assert_eq!(new_count, 0);
          new_count += 1;
          debug_assert_eq!(new_count, 1);
        }
        SyntaxEdit::Update(_) => {}
      }
    }
    debug_assert!(new_count <= 1);
  }

  let mut last_rope: Option<Rope> = None;
  let mut last_payload: Option<String> = None;

  if !pending_edits.is_empty() && matches!(pending_edits[0], SyntaxEdit::New(_))
  {
    match &pending_edits[0] {
      SyntaxEdit::New(new) => {
        let payload = new.payload.to_string();
        let new_tree = parser.parse(&payload, tree.as_ref());
        tree = new_tree;
        editing_version = new.version;
        // trace!(
        //   "Parsed new tree:{:?}, editing_version:{:?}",
        //   tree
        //     .clone()
        //     .map(|t| t.root_node().to_string())
        //     .unwrap_or("None".to_string()),
        //   editing_version
        // );
        last_rope = Some(new.payload.clone());
        last_payload = Some(payload);
      }
      _ => unreachable!(),
    }
  }

  let mut last_update: Option<&SyntaxEditUpdate> = None;
  for (i, edit) in pending_edits.iter().enumerate() {
    if matches!(edit, SyntaxEdit::New(_)) {
      debug_assert_eq!(i, 0);
      continue;
    }
    match edit {
      SyntaxEdit::Update(update) => {
        debug_assert!(tree.is_some());
        if let Some(ref mut tree1) = tree {
          tree1.edit(&update.input);
        }
        last_update = Some(update);
      }
      SyntaxEdit::New(_) => unreachable!(),
    }
  }

  if let Some(last_update) = last_update {
    let payload = last_update.payload.to_string();
    let new_tree = parser.parse(&payload, tree.as_ref());
    tree = new_tree;
    editing_version = last_update.version;
    // trace!(
    //   "Parsed update tree:{:?}, editing_version:{:?}",
    //   tree
    //     .clone()
    //     .map(|t| t.root_node().to_string())
    //     .unwrap_or("None".to_string()),
    //   editing_version
    // );
    last_rope = Some(last_update.payload.clone());
    last_payload = Some(payload);
  }

  (tree, editing_version, last_rope, last_payload)
}

fn convert_ts_byte(rope: &Rope, byte_idx: usize) -> usize {
  debug_assert!(rope.try_byte_to_char(byte_idx).is_ok());
  rope.byte_to_char(byte_idx)
}

fn convert_ts_point(rope: &Rope, point: &tree_sitter::Point) -> (usize, usize) {
  let line_idx = point.row;
  debug_assert!(rope.get_line(line_idx).is_some());
  let line = rope.line(line_idx);
  debug_assert!(line.try_byte_to_char(point.column).is_ok());
  let char_idx = line.byte_to_char(point.column);
  (line_idx, char_idx)
}

/// Here is a really trade-off, I mean we have two solutions when querying
/// syntax highlight colors for a buffer:
///
/// 1. Execute the `QueryCursor::matches` on each viewport/window on every
///    TUI frame.
///    - Pros: The viewport is only a very small part compared with the
///      whole buffer, it gives us smaller problem scale and shorter
///      response time.
///    - Cons: The `QueryCursor::matches` needs to pass the buffer text
///      payload as a `&[u8]` (i.e. `&str`) type. But we are using `Rope` as
///      our text backend, which means we will have to convert (part of) the
///      text to a `String` via something like `to_string` API, which leads
///      to massive memory allocation on each frame.
/// 2. Execute the `QueryCursor::matches` on the whole buffer when syntax
///    parser just finishes its parsing.
///    - Pros: Both syntax parsing and highlight querying are done in
///      background job, the CPU workload of TUI rendering on every frame is
///      reduced, it gives us better performance.
///    - Cons: The `QueryCursor::matches` runs longer because the problem
///      scale becomes larger, since we are querying the whole buffer,
///      instead of a window/viewport. This leads to longer response time,
///      i.e. for a very big buffer, user will wait longer time to get the
///      latest highlights after some editings.
///
/// NOTE: Make this method public only for testing purpose.
pub fn _query(
  tree: &Option<Tree>,
  text_rope: &Option<Rope>,
  text_payload: &Option<String>,
  highlight_query: &Option<TreeSitterQueryArc>,
) -> Option<SyntaxCaptureArc> {
  let mut query_cursor = QueryCursor::new();
  if let Some(syn_tree) = tree
    && let Some(syn_highlight_query) = highlight_query
    && let Some(text_rope) = text_rope
    && let Some(text_payload) = text_payload
  {
    debug_assert_eq!(&text_rope.to_string(), text_payload);
    query_cursor.set_byte_range(0..usize::MAX);
    let mut matches = query_cursor.matches(
      syn_highlight_query,
      syn_tree.root_node(),
      text_payload.as_bytes(),
    );
    let mut nodes: SyntaxCaptureMap = FoldMap::new();
    while let Some(mat) = matches.next() {
      for cap in mat.captures {
        let index = cap.index;
        let name = syn_highlight_query.capture_names()[index as usize];
        let range = cap.node.range();
        trace!(
          "Captured highlight {}: name:{:?}, range:{:?}",
          index, name, range
        );
        debug_assert!(text_rope.get_line(range.start_point.row).is_some());
        debug_assert!(
          text_rope
            .line(range.start_point.row)
            .try_byte_to_char(range.start_point.column)
            .is_ok()
        );
        let (start_line_idx, start_char_idx) =
          convert_ts_point(text_rope, &range.start_point);
        let key = SyntaxCapturePoint {
          line_idx: start_line_idx,
          char_idx: start_char_idx,
        };
        nodes.entry(key).or_insert(SyntaxCaptureMultipleValues {
          values: vec![],
          max_end_char: None,
          max_end_point: None,
        });
        let absolute_start_char_idx =
          convert_ts_byte(text_rope, range.start_byte);
        let absolute_end_char_idx = convert_ts_byte(text_rope, range.end_byte);
        let (end_line_idx, end_char_idx) =
          convert_ts_point(text_rope, &range.end_point);
        let end_point = SyntaxCapturePoint {
          line_idx: end_line_idx,
          char_idx: end_char_idx,
        };
        let val = nodes.get_mut(&key).unwrap();
        val.values.push(SyntaxCaptureValue {
          index,
          name: name.to_compact_string(),
          range: SyntaxCaptureRange {
            start_char: absolute_start_char_idx,
            end_char: absolute_end_char_idx,
            start_point: SyntaxCapturePoint {
              line_idx: start_line_idx,
              char_idx: start_char_idx,
            },
            end_point,
          },
        });
        val.max_end_char = Some(
          val
            .max_end_char
            .map(|c| std::cmp::max(c, absolute_end_char_idx))
            .unwrap_or(absolute_end_char_idx),
        );
        val.max_end_point = Some(
          val
            .max_end_point
            .map(|p| std::cmp::max(p, end_point))
            .unwrap_or(end_point),
        );
      }
    }
    Some(SyntaxCapture::to_arc(SyntaxCapture::new(nodes)))
  } else {
    None
  }
}

pub fn parse_and_query(
  ts_parser: TreeSitterParserArc,
  old_ts_tree: Option<Tree>,
  ts_highlight_query: Option<TreeSitterQueryArc>,
  pending_edits: Vec<SyntaxEdit>,
) -> (Option<Tree>, isize, Option<SyntaxCaptureArc>) {
  let (new_ts_tree, editing_version, text_rope, text_payload) =
    _parse(ts_parser, old_ts_tree, pending_edits);
  let highlight_capture =
    _query(&new_ts_tree, &text_rope, &text_payload, &ts_highlight_query);
  (new_ts_tree, editing_version, highlight_capture)
}
