//! Tree listings: names and content checksums, no timestamps (BR2.5, BR6.4).

use crate::sha256::sha256_hex;
use std::fs;
use std::io;
use std::path::Path;

/// One line per entry under `dir`, sorted by the bytes of the relative path:
/// `d <path>`, `f <sha256> <path>` or `l <path>`. Modification times, modes
/// and owners are not part of the listing.
pub fn tree_listing(dir: &Path) -> io::Result<String> {
    let mut lines: Vec<(String, String)> = Vec::new();
    walk(dir, "", &mut lines)?;
    lines.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    Ok(lines.into_iter().map(|(_, line)| line + "\n").collect())
}

fn walk(dir: &Path, prefix: &str, out: &mut Vec<(String, String)>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let rel = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            out.push((rel.clone(), format!("l {rel}")));
        } else if kind.is_dir() {
            out.push((rel.clone(), format!("d {rel}")));
            walk(&entry.path(), &rel, out)?;
        } else {
            let sum = sha256_hex(&fs::read(entry.path())?);
            out.push((rel.clone(), format!("f {sum} {rel}")));
        }
    }
    Ok(())
}
