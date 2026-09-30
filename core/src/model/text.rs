//! Rules for user-typed text: tags, file-name slugs, and headings inside descriptions and comments.

/// Slugs in file names are cut at this many characters.
const MAX_SLUG_LEN: usize = 40;

/// Splits on commas/whitespace, drops a leading `#`, lowercases and dedupes.
pub fn parse_tags(s: &str) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    for raw in s.split(|c: char| c == ',' || c.is_whitespace()) {
        let tag = raw.trim_start_matches('#').to_lowercase();
        if !tag.is_empty() && !tags.contains(&tag) {
            tags.push(tag);
        }
    }
    tags
}

/// `Fix: prod login!` → `fix-prod-login`; ASCII only so the name works on every file system.
pub fn slugify(title: &str) -> String {
    let mut slug = String::new();
    for c in title.chars() {
        if slug.len() >= MAX_SLUG_LEN {
            break;
        }
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() { "task".to_string() } else { slug.to_string() }
}

/// In a task file `## ` starts a section and, inside Comments, `### ` starts a comment.
/// Headings typed into a description or comment are pushed below those levels so they
/// can't break the file. Returns the text and whether anything was changed.
pub fn sanitize_description(text: &str) -> (String, bool) {
    demote(text, &["## "], "### ")
}

/// See [`sanitize_description`].
pub fn sanitize_comment(text: &str) -> (String, bool) {
    demote(text, &["## ", "### "], "#### ")
}

/// Rewrites lines starting with any of `prefixes` to start with `to` instead.
fn demote(text: &str, prefixes: &[&str], to: &str) -> (String, bool) {
    let mut changed = false;
    let lines: Vec<String> = text
        .lines()
        .map(|line| match prefixes.iter().find_map(|p| line.strip_prefix(p)) {
            Some(rest) => {
                changed = true;
                format!("{to}{rest}")
            }
            None => line.to_string(),
        })
        .collect();
    (lines.join("\n").trim().to_string(), changed)
}

#[cfg(test)]
#[path = "../../tests/model/text.rs"]
mod tests;
