// src/parser/ast.rs
//
// Compatibility shim: re-export the AST definitions from the crate root.
//
// The real AST types now live in src/ast.rs as `crate::ast::*`.
// This module exists so older `crate::parser::ast::...` paths keep working
// without changing all imports.

pub use crate::ast::*;
