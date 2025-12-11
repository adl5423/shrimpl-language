// src/lint.rs
//
// Simple lint wrapper that reuses docs diagnostics and adds a few
// text-based checks (trailing whitespace, tabs).

use crate::docs;
use crate::parser::ast::Program;
use std::fs;

pub fn run_lint(program: &Program) -> bool {
    let mut had_issue = false;

    // Structural diagnostics from docs module.
    let diags = docs::build_diagnostics(program);
    if let Some(arr) = diags.as_array() {
        for d in arr {
            had_issue = true;
            println!("{}", serde_json::to_string_pretty(d).unwrap());
        }
    }

    // Lightweight source-based lints.
    if let Ok(src) = fs::read_to_string("app.shr") {
        for (idx, line) in src.lines().enumerate() {
            let line_no = idx + 1;
            if line.contains('\t') {
                had_issue = true;
                println!(
                    r#"{{"kind":"warning","scope":"format","line":{},"message":"tab character found (use spaces for indentation)"}}"#,
                    line_no
                );
            }
            if line.ends_with(' ') {
                had_issue = true;
                println!(
                    r#"{{"kind":"warning","scope":"format","line":{},"message":"trailing whitespace"}} "#,
                    line_no
                );
            }
        }
    }

    if !had_issue {
        println!("[shrimpl lint] no issues found");
    }

    had_issue
}
