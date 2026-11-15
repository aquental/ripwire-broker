//! Reading `.github/workflows/*.yml` in tests, by line: a job's block, its code without comments,
//! the job names, one step. A property written only in a comment is not a property, so the
//! assertions run on `code`.
use std::path::Path;

/// The text of `.github/workflows/<name>`.
pub fn read(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(".github/workflows")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn is_job_start(line: &str) -> bool {
    line.starts_with("  ") && !line.starts_with("   ") && !line[2..].starts_with('#')
}

/// The names of the jobs, in order: the keys at two spaces under `jobs:`.
pub fn job_names(yml: &str) -> Vec<String> {
    yml.lines()
        .skip_while(|l| *l != "jobs:")
        .skip(1)
        .take_while(|l| l.is_empty() || l.starts_with(' ') || l.starts_with('#'))
        .filter(|l| is_job_start(l) && l.trim_end().ends_with(':'))
        .map(|l| l.trim().trim_end_matches(':').to_string())
        .collect()
}

/// A job's own lines, raw, from `  <job>:` to the next job.
pub fn job_block(yml: &str, job: &str) -> Vec<String> {
    let header = format!("  {job}:");
    yml.lines()
        .skip_while(|l| *l != header)
        .enumerate()
        .take_while(|(i, l)| *i == 0 || !is_job_start(l))
        .map(|(_, l)| l.to_owned())
        .collect()
}

/// Lines without their comments, blank ones dropped, indentation kept.
pub fn code_lines(block: &[String]) -> Vec<String> {
    block
        .iter()
        .map(|l| without_comment(l).trim_end().to_string())
        .filter(|l| !l.trim().is_empty())
        .collect()
}

/// A job's code: [`job_block`] through [`code_lines`].
pub fn job_code(yml: &str, job: &str) -> Vec<String> {
    code_lines(&job_block(yml, job))
}

/// A line up to its comment: a `#` at the start or after a blank, outside quotes.
pub fn without_comment(line: &str) -> &str {
    let (mut single, mut double, mut previous) = (false, false, ' ');
    for (i, c) in line.char_indices() {
        match c {
            '\'' if !double => single = !single,
            '"' if !single => double = !double,
            '#' if !single && !double && previous.is_whitespace() => return &line[..i],
            _ => {}
        }
        previous = c;
    }
    line
}

/// The step of `code` holding a line that contains `needle`: from its `- ` line to the next
/// step's.
pub fn step(code: &[String], needle: &str) -> Vec<String> {
    let at = code
        .iter()
        .position(|l| l.contains(needle))
        .unwrap_or_else(|| panic!("no `{needle}` in:\n{}", code.join("\n")));
    let starts = |l: &String| l.trim_start().starts_with("- ");
    let start = code[..=at]
        .iter()
        .rposition(starts)
        .expect("the step's first line");
    let end = code[at + 1..]
        .iter()
        .position(starts)
        .map_or(code.len(), |i| at + 1 + i);
    code[start..end].to_vec()
}

/// Whether a line is an `if:`, also as a step's first key (`- if:`).
pub fn is_condition(line: &str) -> bool {
    let key = line.trim_start();
    key.strip_prefix("- ").unwrap_or(key).starts_with("if:")
}
