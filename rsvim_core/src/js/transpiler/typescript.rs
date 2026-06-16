//! Typescript transpiler.

use crate::prelude::*;
use swc_common::FileName;
use swc_common::GLOBALS;
use swc_common::Globals;
use swc_common::Mark;
use swc_common::SourceMap;
use swc_common::sync::Lrc;
use swc_ecma_ast::EsVersion;
use swc_ecma_codegen::Emitter;
use swc_ecma_codegen::text_writer::JsWriter;
use swc_ecma_parser::Parser;
use swc_ecma_parser::StringInput;
use swc_ecma_parser::Syntax;
use swc_ecma_parser::TsSyntax;
use swc_ecma_parser::lexer::Lexer;
use swc_ecma_transforms_base::fixer::fixer;
use swc_ecma_transforms_base::hygiene::hygiene;
use swc_ecma_transforms_base::resolver;
// use swc_ecma_transforms_react::react;
// use swc_ecma_transforms_react::Options;
use compact_str::ToCompactString;
use swc_ecma_transforms_typescript::strip;

pub struct TypeScript;

impl TypeScript {
  /// Compiles TypeScript code into JavaScript.
  pub fn compile(filename: Option<&str>, source: &str) -> TheResult<String> {
    let globals = Globals::default();
    let cm: Lrc<SourceMap> = Default::default();

    let filename = match filename {
      Some(filename) => FileName::Custom(filename.into()),
      None => FileName::Anon,
    };

    let fm = cm.new_source_file(filename.into(), source.to_string());

    // Initialize the TypeScript lexer.
    let lexer = Lexer::new(
      Syntax::Typescript(TsSyntax {
        tsx: true,
        decorators: true,
        no_early_errors: true,
        ..Default::default()
      }),
      // NOTE: Always use "esnext" version.
      EsVersion::EsNext,
      StringInput::from(&*fm),
      None,
    );

    let mut parser = Parser::new_from(lexer);

    let program = match parser.parse_program() {
      Ok(module) => module,
      Err(e) => {
        return Err(TheErr::CompileTypeScriptFailed(
          e.kind().msg().to_compact_string(),
        ));
      }
    };

    // This is where we're gonna store the JavaScript output.
    let mut buffer = vec![];

    GLOBALS.set(&globals, || {
      // Apply the rest SWC transforms to generated code.
      let program = program
        .apply(&mut resolver(Mark::new(), Mark::new(), true))
        .apply(&mut strip(Mark::new(), Mark::new()))
        .apply(&mut hygiene())
        .apply(&mut fixer(None));

      {
        // NOTE: Always use "esnext" version.
        let cfg =
          swc_ecma_codegen::Config::default().with_target(EsVersion::EsNext);
        let mut emitter = Emitter {
          cfg,
          cm: cm.clone(),
          comments: None,
          wr: JsWriter::new(cm, "\n", &mut buffer, None),
        };

        emitter.emit_program(&program).unwrap();
      }
    });

    Ok(String::from_utf8_lossy(&buffer).to_string())
  }
}
