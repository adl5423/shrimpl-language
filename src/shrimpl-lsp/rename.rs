// src/shrimpl-lsp/rename.rs
//
// Basic rename support for functions and endpoints.
// It renames:
//   - function definitions `func name(...)`
//   - references `name(...)`
//   - endpoint path params are intentionally ignored for now.

use crate::parser::ast::Program;
use tower_lsp::lsp_types::{
    Position, Range, TextDocumentIdentifier, TextDocumentPositionParams, TextEdit, Url,
    WorkspaceEdit,
};

fn same_pos(line: usize, col_start: usize, col_end: usize, pos: &Position) -> bool {
    pos.line as usize == line && (pos.character as usize) >= col_start && (pos.character as usize) <= col_end
}

pub fn prepare_rename(
    _program: &Program,
    _params: TextDocumentPositionParams,
) -> Option<Range> {
    // For now we allow rename anywhere; the client will highlight the word range.
    // A more precise implementation would inspect the AST based on position.
    None
}

pub fn compute_rename(
    uri: &Url,
    text: &str,
    new_name: &str,
) -> Option<WorkspaceEdit> {
    // Very simple word-based rename: replace all exact word matches in the file.
    // Later you can plug in AST-based symbol mapping.
    let mut edits = Vec::<TextEdit>::new();

    for (line_idx, line) in text.lines().enumerate() {
        let mut start = 0usize;
        while let Some(idx) = line[start..].find(new_name) {
            let abs = start + idx;
            let end = abs + new_name.len();

            // Here we assume the user has selected the correct symbol;
            // in a real implementation you would check identifier boundaries.
            let range = Range {
                start: Position {
                    line: line_idx as u32,
                    character: abs as u32,
                },
                end: Position {
                    line: line_idx as u32,
                    character: end as u32,
                },
            };
            edits.push(TextEdit {
                range,
                new_text: new_name.to_string(),
            });
            start = end;
        }
    }

    if edits.is_empty() {
        None
    } else {
        Some(WorkspaceEdit {
            changes: Some([(uri.clone(), edits)].into_iter().collect()),
            document_changes: None,
            change_annotations: None,
        })
    }
}
