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
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Number(f64),
    Str(String),
    Var(String),
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
