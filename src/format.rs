// src/format.rs
//
// Minimal formatter that normalizes indentation (tabs -> two spaces)
// and trims trailing whitespace. It does not reflow code based on AST;
// it is intentionally conservative.

use std::fs;
use std::io::Write;
use std::path::Path;

pub fn format_file_in_place(path: &str) -> std::io::Result<()> {
    let p = Path::new(path);
    let src = fs::read_to_string(p)?;

    let mut out = String::with_capacity(src.len());
    for line in src.lines() {
        let mut s = line.replace('\t', "  ");
        while s.ends_with(' ') {
            s.pop();
        }
        out.push_str(&s);
        out.push('\n');
    }

    let mut file = fs::File::create(p)?;
    file.write_all(out.as_bytes())?;
    println!("[shrimpl format] formatted {}", path);
    Ok(())
}
