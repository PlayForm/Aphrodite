"""CODING_TASK"""

from .types import Conversation, ToolCall, Turn

# ═══════════════════════════════════════════════════════════════════════════════
CODING_TASK = Conversation(
    name="coding_task",
    description="Multi-file Rust refactoring - extract trait, fix errors, run build",
    system_prompt="""You are a Rust programming assistant. The user needs help refactoring a multi-module Rust project. Use tools to read files, make edits, and verify builds. Be thorough and check your work.""",
    turns=[
        # Turn 1: User describes the task
        Turn(
            role="user",
            content="""I have a Rust project with these modules: `src/parser.rs` (tokenizer + AST), `src/codegen.rs` (LLVM IR generation), and `src/main.rs` (CLI driver). The parser and codegen both have their own error types that are almost identical. I want to extract a shared `Error` enum into `src/error.rs` and use it everywhere. Can you help me refactor this?""",
        ),
        # Turn 2: Assistant reads the files
        Turn(
            role="assistant",
            content="Let me read the current files to understand the error types.",
            tool_calls=[
                ToolCall(id="call_1", name="read_file", arguments='{"path": "src/parser.rs"}'),
                ToolCall(id="call_2", name="read_file", arguments='{"path": "src/codegen.rs"}'),
                ToolCall(id="call_3", name="read_file", arguments='{"path": "src/main.rs"}'),
            ],
        ),
        # Turn 3: Tool results - large code files that exercise compression
        Turn(
            role="tool",
            tool_call_id="call_1",
            content="""// src/parser.rs - Tokenizer + AST + Error types (~180 lines)
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
""",
        ),
        Turn(
            role="tool",
            tool_call_id="call_2",
            content="""// src/codegen.rs - LLVM IR generation + Error types (~160 lines)
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
""",
        ),
        Turn(
            role="tool",
            tool_call_id="call_3",
            content="""// src/main.rs - CLI driver (~50 lines)
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
""",
        ),
        # Turn 4: Assistant proposes the refactoring
        Turn(
            role="assistant",
            content="""I can see both `ParserError` (in parser.rs) and `CodegenError` (in codegen.rs) share several overlapping variants: UndefinedVariable, TypeMismatch, TooManyArguments. Let me extract the common ones into a shared `error.rs` module.

Here's my plan:
1. Create `src/error.rs` with a unified `CompileError` enum
2. Keep parser-specific variants in `ParserError` (which wraps `CompileError`)
3. Keep codegen-specific variants in `CodegenError` (which wraps `CompileError`)
4. Update imports in all files
5. Run `cargo build` to verify

Let me make these changes now.""",
        ),
        # Turn 5: User asks to proceed
        Turn(role="user", content="Go ahead with the changes. Also run the build after."),
        # Turn 6: Assistant makes edits
        Turn(
            role="assistant",
            content="Creating the files and running the build.",
            tool_calls=[
                ToolCall(
                    id="call_4",
                    name="write_file",
                    arguments='{"path": "src/error.rs", "content": "// shared error types"}',
                ),
                ToolCall(
                    id="call_5",
                    name="terminal",
                    arguments='{"command": "cd /tmp/test_project && cargo build 2>&1"}',
                ),
            ],
        ),
        # Turn 7: Build output - large to exercise compression
        Turn(
            role="tool",
            tool_call_id="call_5",
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
""",
        ),
        # Turn 8: Assistant fixes build errors
        Turn(
            role="assistant",
            content="""The build shows the inkwell dependency is missing from Cargo.toml and the `span`/`ast` modules need to be created. Let me fix these issues:
1. Add `inkwell` to Cargo.toml dependencies
2. Create the missing `src/ast.rs` and `src/span.rs` stub modules
3. Declare them in `src/main.rs`

Let me fix everything and rebuild.""",
        ),
        # Turn 9: Fixes applied, rebuild
        Turn(
            role="assistant",
            content="",
            tool_calls=[
                ToolCall(
                    id="call_6",
                    name="terminal",
                    arguments='{"command": "cd /tmp/test_project && cargo build 2>&1"}',
                ),
            ],
        ),
        Turn(
            role="tool",
            tool_call_id="call_6",
            content="""   Compiling mylang v0.1.0 (/tmp/test_project)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.34s
""",
        ),
        # Turn 10: Final summary
        Turn(
            role="assistant",
            content="""The refactoring is complete and the project builds successfully. Here's a summary of what was done:

1. **Created `src/error.rs`**: Unified `CompileError` with 7 common variants shared between parser and codegen
2. **Module structure**: `main.rs` now declares `mod error; mod ast; mod span;` alongside existing modules
3. **Clean build**: All warnings resolved, no errors

The shared error type eliminates the duplication between `ParserError` and `CodegenError` and will make future error handling additions much simpler.""",
        ),
    ],
)
