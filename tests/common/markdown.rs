//! Reading Markdown in tests as a reader sees it.

/// The text without `<!-- … -->` comments, which GitHub does not render. Fenced code stays: a
/// command in a ```sh block is part of what the reader is told to run.
pub fn without_comments(doc: &str) -> String {
    let mut out = String::new();
    let mut rest = doc;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        rest = rest[start..]
            .split_once("-->")
            .map_or("", |(_, after)| after);
    }
    out.push_str(rest);
    out
}

/// The anchor GitHub gives a heading: lowercase, every character that is not alphanumeric, a
/// space, `-` or `_` dropped, each space a hyphen.
pub fn slug(heading: &str) -> String {
    heading
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
        .map(|c| if c == ' ' { '-' } else { c })
        .collect()
}

/// The lines of the section under the heading `title` (any level), up to the next heading of any
/// level: a subsection is not part of it.
pub fn section_lines(doc: &str, title: &str) -> std::ops::Range<usize> {
    let lines: Vec<&str> = doc.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.starts_with('#') && l.trim_start_matches('#').trim() == title)
        .unwrap_or_else(|| panic!("no heading {title:?}"));
    let end = lines[start + 1..]
        .iter()
        .position(|l| l.starts_with('#'))
        .map_or(lines.len(), |i| start + 1 + i);
    start..end
}
