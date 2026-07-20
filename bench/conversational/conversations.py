"""
Conversation scripts for benchmarking Aphrodite compression across scenarios.

Each script is a list of turns. Each turn has:
  - role: "user" | "assistant" | "tool"
  - content: the message text
  - tool_calls (optional): tool calls the assistant would make
  - tool_results (optional): results of those tool calls (these are what
    the cache proxy compresses when large enough)

The scripts exercise different content types that trigger different
compression paths: code, diffs, build output, errors, search results,
terminal output, JSON.
"""

from dataclasses import dataclass, field
from typing import Optional

@dataclass
class ToolCall:
    id: str
    name: str
    arguments: str  # JSON string

@dataclass
class Turn:
    role: str  # "system" | "user" | "assistant" | "tool"
    content: str
    tool_calls: Optional[list[ToolCall]] = None
    tool_call_id: Optional[str] = None  # for tool role responses

@dataclass
class Conversation:
    name: str
    description: str
    system_prompt: str
    turns: list[Turn]


# ═══════════════════════════════════════════════════════════════════════════════
# Script 1: Multi-file Refactoring Task
# Exercises: code reading (large file outputs), diff generation,
#            build output, error fixing
# ═══════════════════════════════════════════════════════════════════════════════
CODING_TASK = Conversation(
    name="coding_task",
    description="Multi-file Rust refactoring — extract trait, fix errors, run build",
    system_prompt="""You are a Rust programming assistant. The user needs help refactoring a multi-module Rust project. Use tools to read files, make edits, and verify builds. Be thorough and check your work.""",
    turns=[
        # Turn 1: User describes the task
        Turn(role="user", content="""I have a Rust project with these modules: `src/parser.rs` (tokenizer + AST), `src/codegen.rs` (LLVM IR generation), and `src/main.rs` (CLI driver). The parser and codegen both have their own error types that are almost identical. I want to extract a shared `Error` enum into `src/error.rs` and use it everywhere. Can you help me refactor this?"""),

        # Turn 2: Assistant reads the files
        Turn(role="assistant", content="Let me read the current files to understand the error types.",
             tool_calls=[
                 ToolCall(id="call_1", name="read_file", arguments='{"path": "src/parser.rs"}'),
                 ToolCall(id="call_2", name="read_file", arguments='{"path": "src/codegen.rs"}'),
                 ToolCall(id="call_3", name="read_file", arguments='{"path": "src/main.rs"}'),
             ]),

        # Turn 3: Tool results — large code files that exercise compression
        Turn(role="tool", tool_call_id="call_1",
             content="""// src/parser.rs — Tokenizer + AST + Error types (~180 lines)
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Ident(String),
    Number(i64),
    StringLit(String),
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semicolon,
    Colon,
    Arrow,
    Equals,
    Plus, Minus, Star, Slash,
    EqEq, NotEq, Lt, Gt, LtEq, GtEq,
    And, Or, Not,
    Fn, Let, If, Else, While, Return, Struct, Enum, Match, Pub, Use, Mod, Type, Impl, Trait, For, In, As, Mut, Ref, Self_,
    Eof,
}

#[derive(Debug, Clone)]
pub enum ParserError {
    UnexpectedToken { expected: String, found: Token, line: usize, col: usize },
    UnterminatedString { line: usize, col: usize },
    InvalidNumber { raw: String, line: usize, col: usize },
    UnexpectedEOF { expected: String },
    DuplicateDefinition { name: String, line: usize },
    TypeMismatch { expected: String, found: String, line: usize },
    UndefinedVariable { name: String, line: usize },
    TooManyArguments { fn_name: String, expected: usize, got: usize, line: usize },
    InvalidPattern { line: usize },
    TraitBoundUnsatisfied { ty: String, trait_name: String, line: usize },
    RecursiveType { name: String, line: usize },
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParserError::UnexpectedToken { expected, found, line, col } =>
                write!(f, "line {}:{}: expected {}, found {:?}", line, col, expected, found),
            ParserError::UnterminatedString { line, col } =>
                write!(f, "line {}:{}: unterminated string literal", line, col),
            ParserError::InvalidNumber { raw, line, col } =>
                write!(f, "line {}:{}: invalid number literal '{}'", line, col, raw),
            ParserError::UnexpectedEOF { expected } =>
                write!(f, "unexpected end of file, expected {}", expected),
            ParserError::DuplicateDefinition { name, line } =>
                write!(f, "line {}: duplicate definition of '{}'", line, name),
            ParserError::TypeMismatch { expected, found, line } =>
                write!(f, "line {}: type mismatch: expected {}, found {}", line, expected, found),
            ParserError::UndefinedVariable { name, line } =>
                write!(f, "line {}: undefined variable '{}'", line, name),
            ParserError::TooManyArguments { fn_name, expected, got, line } =>
                write!(f, "line {}: function '{}' expects {} arguments, got {}", line, fn_name, expected, got),
            ParserError::InvalidPattern { line } =>
                write!(f, "line {}: invalid pattern", line),
            ParserError::TraitBoundUnsatisfied { ty, trait_name, line } =>
                write!(f, "line {}: type '{}' does not satisfy trait bound '{}'", line, ty, trait_name),
            ParserError::RecursiveType { name, line } =>
                write!(f, "line {}: recursive type '{}' is not allowed", line, name),
        }
    }
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    errors: Vec<ParserError>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0, errors: Vec::new() }
    }

    fn peek(&self) -> Option<&Token> { self.tokens.get(self.pos) }
    fn advance(&mut self) -> Option<Token> {
        let tok = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        tok
    }

    fn expect(&mut self, expected: Token) -> Result<Token, ParserError> {
        match self.advance() {
            Some(tok) if tok == expected => Ok(tok),
            Some(tok) => Err(ParserError::UnexpectedToken {
                expected: format!("{:?}", expected),
                found: tok.clone(),
                line: 0, col: self.pos,
            }),
            None => Err(ParserError::UnexpectedEOF { expected: format!("{:?}", expected) }),
        }
    }

    pub fn parse_module(&mut self) -> Result<Vec<crate::ast::Statement>, Vec<ParserError>> {
        let mut stmts = Vec::new();
        while self.peek().is_some() {
            match self.parse_statement() {
                Ok(s) => stmts.push(s),
                Err(e) => self.errors.push(e),
            }
        }
        if self.errors.is_empty() { Ok(stmts) } else { Err(self.errors.clone()) }
    }

    fn parse_statement(&mut self) -> Result<crate::ast::Statement, ParserError> {
        match self.peek() {
            Some(Token::Fn) => self.parse_fn_def(),
            Some(Token::Struct) => self.parse_struct_def(),
            Some(Token::Enum) => self.parse_enum_def(),
            Some(Token::Impl) => self.parse_impl_block(),
            Some(Token::Trait) => self.parse_trait_def(),
            Some(Token::Let) => self.parse_let_stmt(),
            Some(Token::If) => self.parse_if_expr().map(crate::ast::Statement::Expr),
            Some(Token::While) => self.parse_while_loop().map(crate::ast::Statement::Expr),
            Some(_) => self.parse_expr().map(crate::ast::Statement::Expr),
            None => Err(ParserError::UnexpectedEOF { expected: "statement".into() }),
        }
    }

    fn parse_fn_def(&mut self) -> Result<crate::ast::Statement, ParserError> { todo!() }
    fn parse_struct_def(&mut self) -> Result<crate::ast::Statement, ParserError> { todo!() }
    fn parse_enum_def(&mut self) -> Result<crate::ast::Statement, ParserError> { todo!() }
    fn parse_impl_block(&mut self) -> Result<crate::ast::Statement, ParserError> { todo!() }
    fn parse_trait_def(&mut self) -> Result<crate::ast::Statement, ParserError> { todo!() }
    fn parse_let_stmt(&mut self) -> Result<crate::ast::Statement, ParserError> { todo!() }
    fn parse_if_expr(&mut self) -> Result<crate::ast::Expr, ParserError> { todo!() }
    fn parse_while_loop(&mut self) -> Result<crate::ast::Expr, ParserError> { todo!() }
    fn parse_expr(&mut self) -> Result<crate::ast::Expr, ParserError> { todo!() }
}
"""),

        Turn(role="tool", tool_call_id="call_2",
             content="""// src/codegen.rs — LLVM IR generation + Error types (~160 lines)
use std::fmt;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::builder::Builder;
use inkwell::values::{BasicValueEnum, IntValue, FunctionValue};
use inkwell::types::{IntType, BasicTypeEnum};

#[derive(Debug, Clone)]
pub enum CodegenError {
    UndefinedVariable { name: String, span: crate::span::Span },
    TypeMismatch { expected: String, found: String, span: crate::span::Span },
    InvalidBinaryOp { lhs: String, rhs: String, op: String, span: crate::span::Span },
    UndefinedFunction { name: String, span: crate::span::Span },
    TooManyArguments { fn_name: String, expected: usize, got: usize, span: crate::span::Span },
    NotCallable { expr_type: String, span: crate::span::Span },
    ReturnOutsideFunction { span: crate::span::Span },
    BreakOutsideLoop { span: crate::span::Span },
    InvalidLValue { span: crate::span::Span },
    StructFieldNotFound { struct_name: String, field: String, span: crate::span::Span },
    LLVMError { message: String },
    Ice { message: String },
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodegenError::UndefinedVariable { name, span } =>
                write!(f, "{}: undefined variable '{}'", span, name),
            CodegenError::TypeMismatch { expected, found, span } =>
                write!(f, "{}: type mismatch: expected {}, found {}", span, expected, found),
            CodegenError::InvalidBinaryOp { lhs, rhs, op, span } =>
                write!(f, "{}: invalid binary operation '{}' between {} and {}", span, op, lhs, rhs),
            CodegenError::UndefinedFunction { name, span } =>
                write!(f, "{}: undefined function '{}'", span, name),
            CodegenError::TooManyArguments { fn_name, expected, got, span } =>
                write!(f, "{}: function '{}' expects {} arguments, got {}", span, fn_name, expected, got),
            CodegenError::NotCallable { expr_type, span } =>
                write!(f, "{}: type '{}' is not callable", span, expr_type),
            CodegenError::ReturnOutsideFunction { span } =>
                write!(f, "{}: return statement outside function body", span),
            CodegenError::BreakOutsideLoop { span } =>
                write!(f, "{}: break statement outside loop", span),
            CodegenError::InvalidLValue { span } =>
                write!(f, "{}: invalid left-hand side of assignment", span),
            CodegenError::StructFieldNotFound { struct_name, field, span } =>
                write!(f, "{}: struct '{}' has no field '{}'", span, struct_name, field),
            CodegenError::LLVMError { message } =>
                write!(f, "LLVM error: {}", message),
            CodegenError::Ice { message } =>
                write!(f, "internal compiler error: {}", message),
        }
    }
}

pub struct Codegen<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    errors: Vec<CodegenError>,
    current_fn: Option<FunctionValue<'ctx>>,
    variables: Vec<std::collections::HashMap<String, BasicValueEnum<'ctx>>>,
}

impl<'ctx> Codegen<'ctx> {
    pub fn new(context: &'ctx Context, name: &str) -> Self {
        let module = context.create_module(name);
        let builder = context.create_builder();
        Codegen {
            context,
            module,
            builder,
            errors: Vec::new(),
            current_fn: None,
            variables: vec![std::collections::HashMap::new()],
        }
    }

    pub fn compile_module(&mut self, stmts: &[crate::ast::Statement]) -> Result<(), Vec<CodegenError>> {
        for stmt in stmts {
            if let Err(e) = self.compile_statement(stmt) {
                self.errors.push(e);
            }
        }
        if self.errors.is_empty() { Ok(()) } else { Err(self.errors.clone()) }
    }

    fn compile_statement(&mut self, stmt: &crate::ast::Statement) -> Result<(), CodegenError> { todo!() }
    pub fn module(&self) -> &Module<'ctx> { &self.module }
    pub fn verify(&self) -> Result<(), String> {
        self.module.verify().map_err(|e| e.to_string())
    }
}
"""),

        Turn(role="tool", tool_call_id="call_3",
             content="""// src/main.rs — CLI driver (~50 lines)
use std::env;
use std::fs;
use std::process;

mod parser;
mod codegen;
mod ast;
mod span;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <source-file>", args[0]);
        process::exit(1);
    }

    let source = match fs::read_to_string(&args[1]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading '{}': {}", args[1], e);
            process::exit(1);
        }
    };

    // Tokenize (placeholder)
    let tokens = Vec::new();

    // Parse
    let mut parser = parser::Parser::new(tokens);
    let ast = match parser.parse_module() {
        Ok(ast) => ast,
        Err(errors) => {
            for e in &errors {
                eprintln!("parse error: {}", e);
            }
            process::exit(1);
        }
    };

    // Codegen
    let context = inkwell::context::Context::create();
    let mut cg = codegen::Codegen::new(&context, "module");
    if let Err(errors) = cg.compile_module(&ast) {
        for e in &errors {
            eprintln!("codegen error: {}", e);
        }
        process::exit(1);
    }

    if let Err(e) = cg.verify() {
        eprintln!("LLVM verification failed: {}", e);
        process::exit(1);
    }

    cg.module().print_to_file("output.ll").unwrap();
    println!("Compiled to output.ll");
}
"""),

        # Turn 4: Assistant proposes the refactoring
        Turn(role="assistant", content="""I can see both `ParserError` (in parser.rs) and `CodegenError` (in codegen.rs) share several overlapping variants: UndefinedVariable, TypeMismatch, TooManyArguments. Let me extract the common ones into a shared `error.rs` module.

Here's my plan:
1. Create `src/error.rs` with a unified `CompileError` enum
2. Keep parser-specific variants in `ParserError` (which wraps `CompileError`)
3. Keep codegen-specific variants in `CodegenError` (which wraps `CompileError`)
4. Update imports in all files
5. Run `cargo build` to verify

Let me make these changes now."""),

        # Turn 5: User asks to proceed
        Turn(role="user", content="Go ahead with the changes. Also run the build after."),

        # Turn 6: Assistant makes edits
        Turn(role="assistant", content="Creating the files and running the build.",
             tool_calls=[
                 ToolCall(id="call_4", name="write_file", arguments='{"path": "src/error.rs", "content": "// shared error types"}'),
                 ToolCall(id="call_5", name="terminal", arguments='{"command": "cd /tmp/test_project && cargo build 2>&1"}'),
             ]),

        # Turn 7: Build output — large to exercise compression
        Turn(role="tool", tool_call_id="call_5",
             content="""   Compiling mylang v0.1.0 (/tmp/test_project)
warning: unused import: `std::fmt`
 --> src/codegen.rs:2:5
  |
2 | use std::fmt;
  |     ^^^^^^^^
  |
  = note: `#[warn(unused_imports)]` on by default

warning: unused variable: `tokens`
  --> src/main.rs:30:9
   |
30 |     let tokens = Vec::new();
   |         ^^^^^^ help: if this is intentional, prefix with an underscore: `_tokens`
   |
   = note: `#[warn(unused_variables)]` on by default

warning: unused import: `BasicTypeEnum`
 --> src/codegen.rs:8:25
  |
8 | use inkwell::types::{IntType, BasicTypeEnum};
  |                              ^^^^^^^^^^^^^^

error[E0433]: failed to resolve: use of undeclared crate or module `inkwell`
 --> src/codegen.rs:3:5
  |
3 | use inkwell::context::Context;
  |     ^^^^^^^ use of undeclared crate or module `inkwell`

error[E0433]: failed to resolve: use of undeclared crate or module `inkwell`
 --> src/main.rs:53:19
  |
53 |     let context = inkwell::context::Context::create();
   |                   ^^^^^^^ use of undeclared crate or module `inkwell`

error[E0432]: unresolved import `crate::span`
 --> src/codegen.rs:10:35
  |
10 |     UndefinedVariable { name: String, span: crate::span::Span },
   |                                              ^^^^ could not find `span` in the crate root

error[E0432]: unresolved import `crate::ast`
 --> src/codegen.rs:67:37
  |
67 |     pub fn compile_module(&mut self, stmts: &[crate::ast::Statement]) -> Result<(), Vec<CodegenError>> {
   |                                                ^^^^ could not find `ast` in the crate root

Some errors have detailed explanations: E0432, E0433.
For more information about an error, try `rustc --explain E0432`.
warning: `mylang` (bin "mylang") generated 3 warnings
error: could not compile `mylang` (bin "mylang") due to 4 previous errors; 3 warnings emitted
"""),

        # Turn 8: Assistant fixes build errors
        Turn(role="assistant", content="""The build shows the inkwell dependency is missing from Cargo.toml and the `span`/`ast` modules need to be created. Let me fix these issues:
1. Add `inkwell` to Cargo.toml dependencies
2. Create the missing `src/ast.rs` and `src/span.rs` stub modules
3. Declare them in `src/main.rs`

Let me fix everything and rebuild."""),

        # Turn 9: Fixes applied, rebuild
        Turn(role="assistant", content="",
             tool_calls=[
                 ToolCall(id="call_6", name="terminal", arguments='{"command": "cd /tmp/test_project && cargo build 2>&1"}'),
             ]),

        Turn(role="tool", tool_call_id="call_6",
             content="""   Compiling mylang v0.1.0 (/tmp/test_project)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.34s
"""),

        # Turn 10: Final summary
        Turn(role="assistant", content="""The refactoring is complete and the project builds successfully. Here's a summary of what was done:

1. **Created `src/error.rs`**: Unified `CompileError` with 7 common variants shared between parser and codegen
2. **Module structure**: `main.rs` now declares `mod error; mod ast; mod span;` alongside existing modules
3. **Clean build**: All warnings resolved, no errors

The shared error type eliminates the duplication between `ParserError` and `CodegenError` and will make future error handling additions much simpler."""),
    ]
)


# ═══════════════════════════════════════════════════════════════════════════════
# Script 2: Large Codebase Exploration
# Exercises: search_files, read_file (multiple large files), catalog usage
# ═══════════════════════════════════════════════════════════════════════════════
EXPLORATION_TASK = Conversation(
    name="exploration_task",
    description="Explore an unfamiliar codebase — search, read many files, understand architecture",
    system_prompt="""You are a software exploration assistant. Help the user understand an unfamiliar codebase by searching for relevant code, reading files, and explaining the architecture. Use search and read tools systematically.""",
    turns=[
        Turn(role="user", content="""I just joined a project and need to understand its HTTP proxy implementation. The codebase is at `/tmp/proxy_project/`. It seems to handle API requests, compression, and has multiple proxy modes. Can you explore it and give me an architectural overview? Start by finding the main entry points."""),

        Turn(role="assistant", content="Let me explore the project structure and find the key files.",
             tool_calls=[
                 ToolCall(id="call_e1", name="search_files", arguments='{"path": "/tmp/proxy_project", "pattern": "*.rs", "target": "files"}'),
             ]),

        Turn(role="tool", tool_call_id="call_e1",
             content="""Found 47 Rust source files:

./src/main.rs (2.1 KB)
./src/lib.rs (4.8 KB)
./src/proxy.rs (24.3 KB)
./src/handlers/mod.rs (1.2 KB)
./src/handlers/openai.rs (8.7 KB)
./src/handlers/anthropic.rs (6.2 KB)
./src/handlers/gemini.rs (4.1 KB)
./src/handlers/bedrock.rs (5.3 KB)
./src/handlers/batch.rs (3.9 KB)
./src/handlers/streaming.rs (7.4 KB)
./src/compression/mod.rs (2.1 KB)
./src/compression/engine.rs (12.4 KB)
./src/compression/classifier.rs (5.8 KB)
./src/compression/marker.rs (3.2 KB)
./src/compression/store.rs (8.9 KB)
./src/compression/retrieve.rs (4.5 KB)
./src/compression/catalog.rs (6.7 KB)
./src/compression/inline.rs (2.3 KB)
./src/config.rs (9.1 KB)
./src/config_loader.rs (5.4 KB)
./src/session.rs (7.8 KB)
./src/state.rs (4.2 KB)
./src/flow.rs (6.3 KB)
./src/hooks.rs (3.1 KB)
./src/setup.rs (2.8 KB)
./src/preview.rs (5.9 KB)
./src/directives.rs (4.7 KB)
./src/resolve.rs (3.3 KB)
./src/prefetch.rs (6.1 KB)
./src/marker.rs (2.2 KB)
./src/poll_worker.rs (3.8 KB)
./src/struct_extract.rs (5.5 KB)
./src/stage2.rs (4.9 KB)
./src/catalog.rs (3.6 KB)
./src/tools/mod.rs (1.8 KB)
./src/tools/aphrodite_catalog.rs (2.4 KB)
./src/tools/aphrodite_compress.rs (3.7 KB)
./src/tools/aphrodite_retrieve.rs (2.9 KB)
./src/tools/aphrodite_search.rs (2.2 KB)
./src/tools/aphrodite_stats.rs (1.8 KB)
./src/tools/aphrodite_test.rs (3.1 KB)
./src/tools/aphrodite_diff.rs (1.5 KB)
./src/tools/aphrodite_files.rs (1.4 KB)
./src/tools/aphrodite_prefetch.rs (2.6 KB)
./src/tools/aphrodite_rebuild.rs (1.7 KB)
./src/tools/aphrodite_directive.rs (2.0 KB)
./src/tools/aphrodite_reclassify.rs (1.9 KB)

Total: 47 files, ~240 KB of Rust source"""),

        Turn(role="assistant", content="Good, I can see the project structure. Let me read the main entry point, the proxy module, and the config to understand the core architecture.",
             tool_calls=[
                 ToolCall(id="call_e2", name="read_file", arguments='{"path": "/tmp/proxy_project/src/main.rs"}'),
                 ToolCall(id="call_e3", name="read_file", arguments='{"path": "/tmp/proxy_project/src/proxy.rs", "limit": 200}'),
                 ToolCall(id="call_e4", name="read_file", arguments='{"path": "/tmp/proxy_project/src/config.rs"}'),
             ]),

        Turn(role="tool", tool_call_id="call_e2",
             content="""// src/main.rs — Binary entry point for the Aphrodite proxy
//
// Two modes: `cache` (compress tool results → CCR) and `token`
// (compress chat completions → CCR offload). Both accept the
// standard /v1/chat/completions endpoint and relay to upstream.
//
// Usage:
//   aphrodite --mode cache --listen 127.0.0.1:9797  (tool relay + cache proxy)
//   aphrodite --mode token --listen 127.0.0.1:9798  (context window compression)

use std::net::SocketAddr;
use std::sync::Arc;
use clap::Parser;
use tracing_subscriber::{fmt, EnvFilter};
use aphrodite::{config, proxy, session, state};
use aphrodite::config_loader::ConfigSource;

/// CLI arguments parsed by clap — env vars like APHRODITE_MODE /
/// APHRODITE_LISTEN / APHRODITE_API_URL / APHRODITE_API_KEY take
/// precedence over flags; the config file is the fallback.
#[derive(Parser, Debug)]
#[command(name = "aphrodite", version, about)]
struct Cli {
    /// Proxy mode: "cache" (tool-relay + tool-output CCR) or "token" (chat CCR offload)
    #[arg(long, env = "APHRODITE_MODE", default_value = "cache")]
    mode: String,

    /// Listen address (e.g. "127.0.0.1:9797")
    #[arg(long, env = "APHRODITE_LISTEN", default_value = "127.0.0.1:9797")]
    listen: SocketAddr,

    /// Upstream API base URL
    #[arg(long, env = "APHRODITE_API_URL")]
    api_url: Option<String>,

    /// Upstream API key
    #[arg(long, env = "APHRODITE_API_KEY")]
    api_key: Option<String>,

    /// Path to the config TOML file
    #[arg(long, env = "APHRODITE_CONFIG_PATH")]
    config_path: Option<String>,

    /// CCR database path (token mode)
    #[arg(long, env = "APHRODITE_CCR_DB_PATH")]
    ccr_db_path: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize structured JSON logging — RUST_LOG controls filter level.
    // In production, this feeds into the operator's log aggregator.
    fmt().json().with_env_filter(EnvFilter::from_default_env()).init();

    let cli = Cli::parse();

    // Config resolution order (highest wins):
    //   1. CLI flags / env vars
    //   2. aphrodite.toml in the working directory
    //   3. Hard-coded defaults
    let source = config_loader::resolve_source(
        cli.config_path.as_deref(),
        cli.mode.clone(),
    );
    let cfg = config::load(source, &cli).await?;

    // Shared application state — one instance per process.
    // In `cache` mode this is just a HashMap; in `token` mode it
    // wraps a SQLite-backed CCR store with TTL-based expiry.
    let state = state::AppState::new(&cfg).await?;

    // Session manager — handles per-connection session state,
    // including the catalog of compressed content available to
    // each conversation.
    let session_mgr = session::SessionManager::new();

    // Build the Axum router and start the server.
    let app = proxy::build_router(Arc::new(state), Arc::new(session_mgr), &cfg);

    tracing::info!(
        mode = %cfg.mode,
        listen = %cfg.listen,
        api_url = %cfg.api_url,
        "Aphrodite proxy starting"
    );

    let listener = tokio::net::TcpListener::bind(cfg.listen).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
"""),

        Turn(role="tool", tool_call_id="call_e3",
             content="""// src/proxy.rs — HTTP proxy: routes, middleware, compression pipeline
//
// Architecture:
// ┌──────────────┐     POST /v1/chat/completions      ┌──────────────────┐
// │   Client     │ ──────────────────────────────────> │  Aphrodite Proxy │
// │  (Hermes)    │ <────────────────────────────────── │                  │
// └──────────────┘     SSE stream / JSON response      └───────┬──────────┘
//                                                              │
//                                          ┌───────────────────┘
//                                          │ POST /v1/chat/completions
//                                          ▼
//                              ┌───────────────────────┐
//                              │  Upstream API          │
//                              │  (DeepSeek / OpenAI)    │
//                              └───────────────────────┘
//
// Cache mode (port 9797): Intercepts tool outputs > threshold,
//   compresses them to CCR, replaces with markers. The LLM
//   sees <<<CCR:hash|type|size>>> instead of large outputs.
//
// Token mode (port 9798): Monitors context window size.
//   When approaching the LLM's context limit, offloads older
//   messages to CCR. Injects aphrodite_retrieve guidance.
//
// Endpoints:
//   POST /v1/chat/completions  - Main chat endpoint (both modes)
//   POST /ccr/create            - Store content in CCR
//   POST /retrieve              - Retrieve CCR entry by hash
//   GET  /stats                 - Proxy health + compression stats
//   GET  /health                - Liveness check
//   POST /reload                - Hot-reload config

use std::sync::Arc;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json, Response, sse::{Event, Sse}},
    routing::{get, post},
    Router,
};
use futures::stream::Stream;
use tokio::sync::RwLock;
use tower_http::cors::{CorsLayer, Any};

use crate::compression::{self, CompressionEngine};
use crate::config::Config;
use crate::session::SessionManager;
use crate::state::AppState;

// ── Router construction ────────────────────────────────────────

pub fn build_router(
    state: Arc<AppState>,
    sessions: Arc<SessionManager>,
    cfg: &Config,
) -> Router {
    let shared = Arc::new(ProxyState {
        app: state,
        sessions,
        config: cfg.clone(),
        engine: CompressionEngine::new(cfg),
    });

    Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route("/ccr/create", post(ccr_create))
        .route("/retrieve", post(retrieve))
        .route("/ccr/{hash}", delete(ccr_delete))
        .route("/stats", get(stats))
        .route("/health", get(health))
        .route("/reload", post(reload_config))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .with_state(shared)
}

// ── Shared proxy state ────────────────────────────────────────

pub struct ProxyState {
    pub app: Arc<AppState>,
    pub sessions: Arc<SessionManager>,
    pub config: Config,
    pub engine: CompressionEngine,
}

// ── Chat completions handler ──────────────────────────────────
//
// This is the core compression entry point. In token mode:
//   1. Check if context window is near capacity
//   2. If yes, select messages to offload
//   3. Compress them into CCR
//   4. Replace with an offload notice + retrieve guidance
//   5. Relays the modified request to upstream
//   6. Streams the response back
//
// In cache mode:
//   Passes through to upstream without message compression.
//   Tool output compression happens in the Hermes process
//   (which calls /ccr/create directly).

pub async fn chat_completions(
    State(state): State<Arc<ProxyState>>,
    Json(mut body): Json<serde_json::Value>,
) -> impl IntoResponse {
    // ... (200 more lines of handler implementation)
}
"""),

        Turn(role="tool", tool_call_id="call_e4",
             content="""// src/config.rs — Configuration types and loading
//
// Loads from aphrodite.toml, CLI flags, and environment variables.
// Supports hot-reload via POST /reload and filesystem watcher.

use std::net::SocketAddr;
use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Top-level configuration root (maps to aphrodite.toml).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AphroditeConfig {
    /// Multiple proxy definitions (for multi-mode operation)
    #[serde(default)]
    pub proxies: Vec<ProxyConfig>,

    /// Shared defaults applied to all proxies unless overridden
    #[serde(default)]
    pub defaults: DefaultsConfig,

    /// Compression thresholds and engine settings
    #[serde(default)]
    pub compression: CompressionConfig,

    /// Preview template configuration per model family
    #[serde(default)]
    pub previews: PreviewsConfig,

    /// Prompt guidance templates
    #[serde(default)]
    pub prompts: PromptsConfig,

    /// Template strings for CCR markers and previews
    #[serde(default)]
    pub templates: TemplatesConfig,

    /// Behavioral directives
    #[serde(default)]
    pub directives: DirectivesConfig,

    /// Per-turn flow budget
    #[serde(default)]
    pub flow: FlowConfig,
}

/// A single proxy instance definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Proxy name: "cache" or "token"
    pub name: String,

    /// Listen address (e.g., "0.0.0.0:9797")
    pub listen: SocketAddr,

    /// Operating mode: "cache" or "token"
    pub mode: String,

    /// Enable tool relay (forward tool calls to upstream)
    #[serde(default = "default_true")]
    pub tool_relay: bool,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,

    /// Optional per-proxy API URL override
    pub api_url: Option<String>,

    /// Optional per-proxy API key override
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsConfig {
    #[serde(default = "default_api_url")]
    pub api_url: String,

    #[serde(default = "default_model")]
    pub model: String,

    #[serde(default = "default_ccr_ttl")]
    pub ccr_ttl_seconds: u64,
}

/// Compression thresholds — these are the knobs that determine
/// when and how content gets compressed. Hot-reloadable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Context engine activation threshold (percentage of max context)
    #[serde(default = "default_engine_threshold_pct")]
    pub engine_threshold_pct: u8,

    /// Messages to protect at the start (system prompt + early turns)
    #[serde(default = "default_protect_first")]
    pub engine_protect_first: usize,

    /// Messages to protect at the end (recent conversation)
    #[serde(default = "default_protect_last")]
    pub engine_protect_last: usize,

    /// Minimum messages before compression activates
    #[serde(default = "default_min_msgs")]
    pub engine_min_msgs: usize,

    /// Token proxy threshold in bytes
    #[serde(default = "default_token_threshold")]
    pub tool_threshold_token: usize,

    /// Cache proxy threshold in bytes
    #[serde(default = "default_cache_threshold")]
    pub tool_threshold_cache: usize,

    /// Terminal output threshold
    #[serde(default = "default_terminal_threshold")]
    pub terminal_threshold: usize,

    /// Inline zlib fallback threshold
    #[serde(default = "default_inline_threshold")]
    pub inline_threshold: usize,

    /// Auto-expand markers in responses
    #[serde(default)]
    pub auto_expand: bool,

    /// Number of markers to auto-expand (0 = disabled)
    #[serde(default)]
    pub auto_expand_limit: usize,

    /// Catalog output mode: "compact", "full", or "tool"
    #[serde(default = "default_catalog_mode")]
    pub catalog_mode: String,

    /// Skip CCR for clean outputs (classifier pre-filter)
    #[serde(default = "default_true")]
    pub classifier_poll: bool,

    /// Code type multiplier
    #[serde(default = "default_code_multiplier")]
    pub code_multiplier: f64,

    /// Context engine status flag
    #[serde(default = "default_true")]
    pub context_engine: bool,

    /// Enable prefetch tool
    #[serde(default = "default_true")]
    pub prefetch: bool,

    /// Enable poll worker
    #[serde(default = "default_true")]
    pub poll_worker: bool,
}

// ... (default value functions and remaining config types)
fn default_true() -> bool { true }
fn default_timeout() -> u64 { 120 }
fn default_api_url() -> String { "https://api.deepseek.com".into() }
fn default_model() -> String { "deepseek-v4-pro".into() }
fn default_ccr_ttl() -> u64 { 3600 }
fn default_engine_threshold_pct() -> u8 { 45 }
fn default_protect_first() -> usize { 2 }
fn default_protect_last() -> usize { 5 }
fn default_min_msgs() -> usize { 8 }
fn default_token_threshold() -> usize { 512 }
fn default_cache_threshold() -> usize { 4096 }
fn default_terminal_threshold() -> usize { 1024 }
fn default_inline_threshold() -> usize { 2048 }
fn default_catalog_mode() -> String { "tool".into() }
fn default_code_multiplier() -> f64 { 3.0 }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PreviewsConfig {
    pub model_family: Option<String>,
    pub code_structure_map: Option<bool>,
    pub preview_max_chars: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PromptsConfig {
    pub retrieve_guidance: Option<String>,
    pub ccr_marker_hint: Option<bool>,
    pub catalog_intent_hints: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemplatesConfig {
    pub preview: Option<serde_json::Value>,
    pub marker: Option<serde_json::Value>,
    pub prompts: Option<serde_json::Value>,
    pub reverse: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DirectivesConfig {
    pub active: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowConfig {
    #[serde(default = "default_flow_budget")]
    pub budget_chars: usize,
}

fn default_flow_budget() -> usize { 4000 }

impl Default for FlowConfig {
    fn default() -> Self { FlowConfig { budget_chars: default_flow_budget() } }
}

/// Runtime config (resolved for a single proxy instance)
#[derive(Debug, Clone)]
pub struct Config {
    pub mode: String,
    pub listen: SocketAddr,
    pub api_url: String,
    pub api_key: String,
    pub model: String,
    pub ccr_ttl_seconds: u64,
    pub ccr_db_path: Option<String>,
    pub tool_relay: bool,
    pub timeout: u64,
    pub compression: CompressionConfig,
    pub previews: PreviewsConfig,
    pub prompts: PromptsConfig,
    pub templates: TemplatesConfig,
    pub directives: DirectivesConfig,
    pub flow: FlowConfig,
}

impl Config {
    pub async fn load_from_file(path: &str) -> anyhow::Result<Self> { todo!() }
    pub fn hot_reload(&mut self, new: AphroditeConfig) { todo!() }
}
"""),

        # Architectural summary
        Turn(role="assistant", content="""Here's the architectural overview of this proxy project:

## Aphrodite Proxy Architecture

### Two Operating Modes

1. **Cache Proxy** (port 9797): Compresses tool outputs (file reads, terminal results, search output) into CCR markers. The LLM sees `<<<CCR:hash|type|size>>>` instead of large tool outputs. Includes tool relay — forwards tool calls to the upstream API.

2. **Token Proxy** (port 9798): Monitors context window utilization. When approaching capacity (45% by default), offloads older messages to CCR and injects retrieval guidance. Only the most recent messages stay in context.

### Request Flow (both modes)
```
Client (Hermes) → POST /v1/chat/completions → Aphrodite Proxy
    → [token mode: compress context if needed]
    → POST /v1/chat/completions → Upstream API (DeepSeek)
    → [stream response back to client]
```

### Key Modules

| Module | Role |
|--------|------|
| `proxy.rs` | HTTP server, routing, main handlers |
| `handlers/` | Provider-specific request/response adapters (OpenAI, Anthropic, Gemini, Bedrock) |
| `compression/engine.rs` | Core compression pipeline — classifier + marker generation + retrieval |
| `compression/store.rs` | CCR storage backend (in-memory for cache, SQLite for token) |
| `config.rs` | TOML config loading, hot-reload support |
| `session.rs` | Per-connection session tracking, catalog management |
| `flow.rs` | Per-turn injected context budget management |

### Compression Pipeline
1. **Classifier** (`classifier.rs`): Determines content type (code, diff, build_output, error, JSON, etc.)
2. **Threshold check**: Compares size against per-type thresholds
3. **Marker generation** (`marker.rs`): Creates `<<<CCR:hash|type|size>>>` markers with preview templates
4. **Storage** (`store.rs`): Stores original content, keyed by content hash (BLAKE3)
5. **Retrieval** (`retrieve.rs`): Resolves markers back to original content

The architecture is clean and modular — new API providers can be added by implementing a handler in the `handlers/` directory, and new compression strategies go in `compression/`."""),
    ]
)


# ═══════════════════════════════════════════════════════════════════════════════
# Script 3: Error Debugging Session
# Exercises: error output, terminal, search, iterative fix loop
# ═══════════════════════════════════════════════════════════════════════════════
DEBUGGING_TASK = Conversation(
    name="debugging_task",
    description="Debug a failing build — search for errors, read files, fix, rebuild, iterate",
    system_prompt="""You are a debugging assistant. Help the user diagnose and fix build errors, test failures, and runtime issues. Use search, file reading, and terminal commands systematically. Always verify fixes by re-running the failing command.""",
    turns=[
        Turn(role="user", content="""My Rust project won't compile. I'm getting dozens of errors from `cargo build`. The project is at `/tmp/rust_project/`. Can you help me figure out what's wrong? Start by running the build and then we'll fix the errors one by one."""),

        Turn(role="assistant", content="Let me run the build to see what errors we're dealing with.",
             tool_calls=[
                 ToolCall(id="call_d1", name="terminal", arguments='{"command": "cd /tmp/rust_project && cargo build 2>&1"}'),
             ]),

        Turn(role="tool", tool_call_id="call_d1",
             content="""   Compiling serde v1.0.228
   Compiling serde_json v1.0.150
   Compiling tokio v1.52.3
   Compiling hyper v1.8.1
   Compiling axum v0.8.9
   Compiling myproject v0.2.0 (/tmp/rust_project)

error[E0061]: this function takes 3 arguments but 2 were supplied
  --> src/api/handlers.rs:142:25
   |
142 |     let response = client.send_request(url, payload);
   |                         ^^^^^^^^^^^^^^^^----------- two arguments of type `String` and `Value`
   |                                         |
   |                                         expected 3 arguments
   |
note: associated function defined here
  --> src/api/client.rs:45:12
   |
45 |     pub fn send_request(&self, url: String, payload: Value, timeout: Duration) -> Result<Response> {
   |            ^^^^^^^^^^^^                                   --------------------
help: provide the missing argument
   |
142 |     let response = client.send_request(url, payload, /* Duration */);
   |                                                    +++++++++++++++++

error[E0308]: mismatched types
  --> src/db/repository.rs:89:13
   |
89 |     let user = users.find(id).await?;
   |                ^^^^^^^^^^^^^^^^^^^^^ expected `User`, found `Option<User>`
   |
   = note: expected struct `User`
              found enum `std::option::Option<User>`

error[E0507]: cannot move out of borrowed content
  --> src/service/processor.rs:234:20
   |
234 |     let data = record.data;
   |                    ^^^^^^^^^^^ move occurs because value has type `Vec<Payload>`, which does not implement the `Copy` trait
   |
help: consider cloning the value
   |
234 |     let data = record.data.clone();
   |                            ++++++++

error[E0599]: no method named `to_response` found for struct `ApiError` in the current scope
  --> src/api/errors.rs:67:31
   |
67 |         self.to_response(status)
   |               ^^^^^^^^^^^ method not found in `ApiError`

error[E0277]: the trait bound `MyStruct: Serialize` is not satisfied
  --> src/models/output.rs:12:5
   |
12 |     #[derive(Serialize, Deserialize)]
   |              ^^^^^^^^^ the trait `Serialize` is not implemented for `MyStruct`
   |
   = help: the following other types implement trait `Serialize`:
             bool
             char
             isize
             i8
             i16
             i32
             i64
             i128
           and 250 others
note: required by a bound in `_serde::export::PhantomData`
  --> /Users/nikola/.cargo/registry/src/index.crates.io-6f17d22bba15001f/serde-1.0.228/src/ser/mod.rs:1234:12
   |
1234| pub trait Serialize {

warning: unused import: `std::collections::HashMap`
 --> src/models/output.rs:3:5
  |
3 | use std::collections::HashMap;
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^
  |
  = note: `#[warn(unused_imports)]` on by default

error[E0432]: unresolved import `crate::config::DatabaseSettings`
  --> src/main.rs:8:5
   |
8  | use crate::config::DatabaseSettings;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ no `DatabaseSettings` in `config`

error[E0425]: cannot find value `CONFIG` in this scope
  --> src/main.rs:34:45
   |
34 |     let db_pool = connect_to_database(&CONFIG.database).await?;
   |                                             ^^^^^^ not found in this scope

Some errors have detailed explanations: E0061, E0277, E0308, E0425, E0432, E0507, E0599.
For more information about an error, try `rustc --explain E0061`.
error: could not compile `myproject` (bin "myproject") due to 7 previous errors; 1 warning emitted
"""),

        Turn(role="assistant", content="""We have 7 errors and 1 warning. Let me tackle them in order of dependency (fixing root causes first, since they often cascade). Let me read the affected files:

1. **E0432 + E0425** in `main.rs`: Missing config module — likely the root cause
2. **E0277** in `models/output.rs`: Missing Serialize derive
3. **E0061** in `api/handlers.rs`: Wrong number of arguments
4. **E0308** in `db/repository.rs`: Option unwrapping
5. **E0507** in `service/processor.rs`: Borrow issue
6. **E0599** in `api/errors.rs`: Missing method

Let me read the files to understand each error."""),

        Turn(role="user", content="Yes, please fix them all. I need this building today."),

        Turn(role="assistant", content="Let me read each failing file and fix the issues.",
             tool_calls=[
                 ToolCall(id="call_d2", name="read_file", arguments='{"path": "/tmp/rust_project/src/main.rs"}'),
                 ToolCall(id="call_d3", name="read_file", arguments='{"path": "/tmp/rust_project/src/models/output.rs"}'),
             ]),

        Turn(role="tool", tool_call_id="call_d2",
             content="""// src/main.rs
use std::env;
use tokio;

mod api;
mod db;
mod models;
mod service;
mod config;

use config::DatabaseSettings;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <config-path>", args[0]);
        std::process::exit(1);
    }

    // FIXME: CONFIG is not defined — should load from args[1]
    let config = config::load_from_file(&args[1])?;

    let db_pool = connect_to_database(&CONFIG.database).await?;

    println!("Server starting on port {}", config.server.port);

    Ok(())
}

async fn connect_to_database(settings: &DatabaseSettings) -> Result<sqlx::PgPool, sqlx::Error> {
    sqlx::PgPool::connect(&settings.url).await
}
"""),

        Turn(role="tool", tool_call_id="call_d3",
             content="""// src/models/output.rs
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

// ERROR: MyStruct uses #[derive(Serialize)] but contains a field
// whose type doesn't implement Serialize. The HashMap import is
// unused — the actual HashMap is std's.

pub struct MyStruct {
    pub id: i64,
    pub data: Vec<Payload>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct Payload {
    pub kind: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    pub status: String,
    pub results: Vec<MyStruct>,
}
"""),

        Turn(role="assistant", content="I see the issues. Let me fix all of them now.",
             tool_calls=[
                 ToolCall(id="call_d4", name="patch", arguments='{"path": "/tmp/rust_project/src/main.rs", "mode": "replace", "old_string": "use config::DatabaseSettings;", "new_string": "use config::{DatabaseSettings, Config as AppConfig};"}'),
                 ToolCall(id="call_d5", name="terminal", arguments='{"command": "cd /tmp/rust_project && cargo build 2>&1"}'),
             ]),

        Turn(role="tool", tool_call_id="call_d5",
             content="""   Compiling myproject v0.2.0 (/tmp/rust_project)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.12s
"""),

        Turn(role="assistant", content="""All 7 errors are fixed and the project builds cleanly now. Here's what was changed:

1. **main.rs**: Fixed `CONFIG` → `config` (the loaded config variable)
2. **models/output.rs**: Added `Serialize, Deserialize` derives to `MyStruct` and `Payload`
3. **api/handlers.rs**: Added the missing `timeout` argument to `send_request`
4. **db/repository.rs**: Changed `users.find(id)` to `users.find(id).ok_or(...)` for proper Option handling
5. **service/processor.rs**: Changed `record.data` to `record.data.clone()`
6. **api/errors.rs**: Added the `to_response` method to `ApiError`

The build is green. Your project is ready to use!"""),
    ]
)


# ═══════════════════════════════════════════════════════════════════════════════
# All scripts registry
# ═══════════════════════════════════════════════════════════════════════════════
ALL_CONVERSATIONS = [CODING_TASK, EXPLORATION_TASK, DEBUGGING_TASK]
