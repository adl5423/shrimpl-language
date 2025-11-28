// src/parser/ast.rs
//
// Core AST types for Shrimpl programs.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ServerDecl {
    pub port: u16,
}

#[derive(Debug, Clone)]
pub enum Method {
    Get,
    Post,
}

#[derive(Debug, Clone)]
pub enum BinOp {
    // arithmetic
    Add,
    Sub,
    Mul,
    Div,
    // comparisons
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    // boolean logic
    And,
    Or,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Number(f64),
    Str(String),
    Bool(bool),

    Var(String),

    /// First-class list literal:
    ///
    ///   [1, 2, "x"]
    ///
    /// is represented as:
    ///   List([Number(1.0), Number(2.0), Str("x".into())])
    List(Vec<Expr>),

    /// First-class map/dict literal:
    ///
    ///   { name: "Shrimpl", version: 0.5 }
    ///   { "owner": "Aisen", "year": 2025 }
    ///
    /// Keys are either identifiers or string literals, both normalized
    /// to String here.
    Map(Vec<(String, Expr)>),

    Binary {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
    },

    Call {
        name: String,
        args: Vec<Expr>,
    },

    MethodCall {
        class_name: String,
        method_name: String,
        args: Vec<Expr>,
    },

    /// if / elif / else as an expression
    ///
    /// Example:
    ///
    ///   if x > 0:
    ///       "positive"
    ///   elif x == 0:
    ///       "zero"
    ///   else:
    ///       "negative"
    ///
    /// is represented as:
    ///   branches = [(x > 0, "positive"), (x == 0, "zero")]
    ///   else_branch = Some("negative")
    If {
        branches: Vec<(Expr, Expr)>,
        else_branch: Option<Box<Expr>>,
    },

    /// Safe bounded loop:
    ///
    ///   repeat N times: body_expr
    ///
    /// Evaluates `count` once, coerces to integer N (floor),
    /// executes `body` N times, returns the last value (or "" if N == 0).
    Repeat {
        count: Box<Expr>,
        body: Box<Expr>,
    },
}

#[derive(Debug, Clone)]
pub enum Body {
    TextExpr(Expr),
    JsonRaw(String),
}

#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: Expr,
}

#[derive(Debug, Clone)]
pub struct ClassDef {
    pub name: String,
    pub methods: HashMap<String, FunctionDef>,
}

#[derive(Debug, Clone)]
pub struct EndpointDecl {
    pub method: Method,
    pub path: String,
    pub body: Body,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub server: ServerDecl,
    pub endpoints: Vec<EndpointDecl>,
    pub functions: HashMap<String, FunctionDef>,
    pub classes: HashMap<String, ClassDef>,
}
