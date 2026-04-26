//! WebAssembly bindings for the Fumola interpreter.
//!
//! Narrow API: `eval(source)` takes fumola source as a string and returns
//! a JSON record describing the result. Independent of the `fumola` crate
//! (which pulls in CLI deps that don't compile to wasm); we replicate the
//! tiny parse-then-eval pipeline directly against fumola_parser +
//! fumola_semantics.
//!
//! Build (with wasm-pack):
//!     wasm-pack build --target web --out-dir pkg crates/fumola_wasm
//!
//! Or with cargo + wasm-bindgen-cli:
//!     cargo build --release --target wasm32-unknown-unknown -p fumola_wasm
//!     wasm-bindgen --target web --out-dir pkg \
//!         target/wasm32-unknown-unknown/release/fumola_wasm.wasm

use fumola_parser::parser::ProgParser;
use fumola_parser::parser_types::SyntaxError;
use fumola_semantics::format::format_one_line;
use fumola_semantics::vm_types::{Core, ModuleFileInit, ModuleFileState, ModulePath};
use fumola_semantics::Interruption;
use fumola_syntax::ast::{Dec, Loc, Prog, Source};
use fumola_syntax::lexer::create_token_tree;
use fumola_syntax::lexer_types::{GroupType, Token, TokenTree};
use regex::Regex;
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[derive(Serialize)]
struct EvalOk<'a> {
    ok: bool,
    output: &'a str,
}

#[derive(Serialize)]
struct EvalErr<'a> {
    ok: bool,
    error: &'a str,
}

fn spacify_token_tree(tt: TokenTree) -> TokenTree {
    let re = Regex::new(r"\S").unwrap();
    TokenTree::Token(Loc(
        Token::Space(re.replace_all(&format!("{}", tt), " ").to_string()),
        Source::Unknown,
    ))
}

fn prepare_token_tree(tt: TokenTree) -> TokenTree {
    match tt {
        TokenTree::Token(Loc(ref token, _)) => match token {
            Token::LineComment(_) | Token::BlockComment(_) => spacify_token_tree(tt),
            _ => tt,
        },
        TokenTree::Group(_, GroupType::Comment, _) => spacify_token_tree(tt),
        TokenTree::Group(trees, group, pair) => TokenTree::Group(
            trees.into_iter().map(prepare_token_tree).collect(),
            group,
            pair,
        ),
    }
}

fn parse(input: &str) -> Result<Prog, SyntaxError> {
    let tt = create_token_tree(input).map_err(|_| SyntaxError::Custom {
        message: "Unknown lexer error".to_string(),
    })?;
    let prepared = prepare_token_tree(tt);
    let input_str = format!("{}", prepared);
    ProgParser::new()
        .parse(&line_col::LineColLookup::new(&input_str), &input_str)
        .map_err(SyntaxError::from_parse_error)
}

/// Evaluate a fumola program. Returns a JSON-encoded result string:
///   `{"ok": true,  "output": "<pretty-printed value>"}` on success
///   `{"ok": false, "error":  "<debug-formatted error>"}` on failure
#[wasm_bindgen]
pub fn eval(source: &str) -> String {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();

    let prog = match parse(source) {
        Ok(p) => p,
        Err(e) => {
            let msg = format!("{:?}", e);
            return serde_json::to_string(&EvalErr {
                ok: false,
                error: &msg,
            })
            .unwrap_or_default();
        }
    };

    let mut core = Core::empty();
    match core.eval_prog(prog) {
        Ok(v) => {
            let s = format_one_line(&v);
            serde_json::to_string(&EvalOk {
                ok: true,
                output: &s,
            })
            .unwrap_or_default()
        }
        Err(e) => {
            let msg = format!("{:?}", e);
            serde_json::to_string(&EvalErr {
                ok: false,
                error: &msg,
            })
            .unwrap_or_default()
        }
    }
}

/// Convenience: pretty-print the result (or `Err(...)`) as a plain string.
#[wasm_bindgen(js_name = evalString)]
pub fn eval_string(source: &str) -> String {
    let prog = match parse(source) {
        Ok(p) => p,
        Err(e) => return format!("ParseErr({:?})", e),
    };
    let mut core = Core::empty();
    match core.eval_prog(prog) {
        Ok(v) => format_one_line(&v),
        Err(e) => format!("Err({:?})", e),
    }
}

fn parse_module(content: &str) -> Result<ModuleFileInit, String> {
    let p = parse(content).map_err(|e| format!("{:?}", e))?;
    if p.vec.is_empty() {
        return Err(format!("{:?}", Interruption::MissingModuleDefinition));
    }
    let mut vec = p.vec.clone();
    let last = vec.pop_back();
    match last {
        Some(d) => match &d.0 {
            Dec::LetModule(id, _, dfs) => Ok(ModuleFileInit {
                file_content: content.to_string(),
                outer_decs: vec,
                id: id.clone().map(|i| i.0.id_()),
                fields: dfs.dec_fields().clone(),
            }),
            _ => Err(format!("{:?}", Interruption::NotAModuleDefinition)),
        },
        None => unreachable!(),
    }
}

/// Stateful fumola interpreter handle. Holds a Core, lets JS register
/// modules by path before evaluating expressions that `import` them.
///
/// JS usage:
///     const state = new FumolaState();
///     state.set_module("handle", handleSourceText);
///     const json = state.eval('import H "handle"; H.handle("GET", "/", "")');
#[wasm_bindgen]
pub struct FumolaState {
    core: Core,
}

#[wasm_bindgen]
impl FumolaState {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();
        FumolaState { core: Core::empty() }
    }

    /// Register a module under `local_path`. Pass the file's full source
    /// text (must begin with `module { ... }`). Trailing `.fumola` in the
    /// path is stripped to match the import-statement convention.
    /// Returns "" on success, error string on failure.
    #[wasm_bindgen(js_name = setModule)]
    pub fn set_module(&mut self, local_path: &str, content: &str) -> String {
        let mut local_path = local_path.to_string();
        if local_path.ends_with(".fumola") {
            local_path.truncate(local_path.len() - 7);
        } else if local_path.ends_with(".mo") {
            local_path.truncate(local_path.len() - 3);
        }
        let path = ModulePath {
            package_name: None,
            local_path,
        };
        let init = match parse_module(content) {
            Ok(i) => i,
            Err(e) => return e,
        };
        let _old = self.core.module_files.map.insert(path, ModuleFileState::Init(init));
        String::new()
    }

    /// Evaluate `source`. Returns a JSON-encoded `{ok, output|error}`.
    pub fn eval(&mut self, source: &str) -> String {
        let prog = match parse(source) {
            Ok(p) => p,
            Err(e) => {
                let msg = format!("{:?}", e);
                return serde_json::to_string(&EvalErr { ok: false, error: &msg })
                    .unwrap_or_default();
            }
        };
        match self.core.eval_prog(prog) {
            Ok(v) => {
                let s = format_one_line(&v);
                serde_json::to_string(&EvalOk { ok: true, output: &s })
                    .unwrap_or_default()
            }
            Err(e) => {
                let msg = format!("{:?}", e);
                serde_json::to_string(&EvalErr { ok: false, error: &msg })
                    .unwrap_or_default()
            }
        }
    }
}
