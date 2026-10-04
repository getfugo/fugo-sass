//! HRX archives (<https://github.com/google/hrx>), in which most specs live.

/// The files and directories of an HRX archive (<https://github.com/google/hrx>).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Hrx {
    /// Each file's path in the archive and its contents.
    pub files: Vec<(String, String)>,
    /// The directories the archive declares (`<===> dir/`), without the `/`.
    pub dirs: Vec<String>,
}

/// Parses an HRX archive. The boundary is the archive's first line's `<`, `=`s and `>`; a body
/// ends before the newline that precedes the next boundary, or at the end of the archive.
///
/// # Errors
/// An archive that does not start with a boundary, or an entry with no path after it.
pub fn parse_hrx(text: &str) -> Result<Hrx, String> {
    let bytes = text.as_bytes();
    let blen = match bytes.first() {
        Some(b'<') => {
            let eqs = bytes[1..].iter().take_while(|&&b| b == b'=').count();
            if eqs == 0 || bytes.get(1 + eqs) != Some(&b'>') {
                return Err("the archive does not start with a boundary".to_owned());
            }
            eqs + 2
        }
        _ => return Err("the archive does not start with a boundary".to_owned()),
    };
    let boundary = &text[..blen];
    let is_boundary = |at: usize| {
        text[at..].starts_with(boundary)
            && matches!(bytes.get(at + blen), None | Some(b' ' | b'\n'))
    };
    let mut starts = vec![0];
    for (i, b) in bytes.iter().enumerate() {
        if *b == b'\n' && i + 1 < bytes.len() && is_boundary(i + 1) {
            starts.push(i + 1);
        }
    }
    let mut hrx = Hrx::default();
    for (n, &start) in starts.iter().enumerate() {
        let header_end = text[start..].find('\n').map_or(text.len(), |i| start + i);
        let header = &text[start + blen..header_end];
        let body_start = (header_end + 1).min(text.len());
        let body = match starts.get(n + 1) {
            Some(&next) => &text[body_start..(next - 1).max(body_start)],
            None => &text[body_start..],
        };
        if header.is_empty() {
            continue; // a comment
        }
        let Some(path) = header.strip_prefix(' ') else {
            return Err(format!("expected a path after {boundary:?}"));
        };
        if let Some(dir) = path.strip_suffix('/') {
            hrx.dirs.push(dir.to_owned());
        } else {
            hrx.files.push((path.to_owned(), body.to_owned()));
        }
    }
    Ok(hrx)
}
