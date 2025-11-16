// src/interpreter/mod.rs
//
// Top-level module for the Shrimpl interpreter.
// Re-exports `run` so main.rs can call `interpreter::run(...)`.

pub mod http;
pub mod eval;

pub use http::run;
