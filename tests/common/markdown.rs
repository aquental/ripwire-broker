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
