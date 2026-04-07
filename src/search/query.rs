/// A parsed representation of a user search query.
#[derive(Debug, Default, PartialEq)]
pub struct ParsedQuery {
    /// Remaining tokens joined for fuzzy matching.
    pub fuzzy: String,
    /// Tags extracted from `#tag` tokens.
    pub tags: Vec<String>,
    /// Negation patterns extracted from `!pattern` tokens.
    pub negations: Vec<String>,
    /// Directory prefix extracted from the first `^path` token.
    pub prefix: Option<String>,
    /// Exact match strings extracted from `'term` tokens.
    pub exact: Vec<String>,
}

/// Parse a query string into its structured components.
///
/// Token routing rules (applied per whitespace-separated token):
/// - `\#`, `\!`, `\^`, `\'` — escaped: strip backslash, add literal to fuzzy
/// - `#word`  — tag
/// - `!word`  — negation
/// - `^path`  — directory prefix (only the first occurrence; subsequent ones go to fuzzy)
/// - `'term`  — exact match
/// - anything else — fuzzy
pub fn parse_query(input: &str) -> ParsedQuery {
    let mut tags: Vec<String> = Vec::new();
    let mut negations: Vec<String> = Vec::new();
    let mut prefix: Option<String> = None;
    let mut exact: Vec<String> = Vec::new();
    let mut fuzzy_owned: Vec<String> = Vec::new();

    for token in input.split_whitespace() {
        if let Some(rest) = token.strip_prefix('\\') {
            // Escaped token — strip the leading backslash, send the rest to fuzzy.
            // Only strip one level of escaping for the recognised special chars.
            match rest.chars().next() {
                Some('#') | Some('!') | Some('^') | Some('\'') => {
                    fuzzy_owned.push(rest.to_string());
                }
                _ => {
                    // Backslash before a non-special character: keep as-is.
                    fuzzy_owned.push(token.to_string());
                }
            }
        } else if let Some(tag) = token.strip_prefix('#') {
            tags.push(tag.to_string());
        } else if let Some(neg) = token.strip_prefix('!') {
            negations.push(neg.to_string());
        } else if let Some(path) = token.strip_prefix('^') {
            if prefix.is_none() {
                prefix = Some(path.to_string());
            } else {
                // Subsequent prefix tokens go to fuzzy unchanged.
                fuzzy_owned.push(token.to_string());
            }
        } else if let Some(term) = token.strip_prefix('\'') {
            exact.push(term.to_string());
        } else {
            fuzzy_owned.push(token.to_string());
        }
    }

    let fuzzy = fuzzy_owned.join(" ");

    ParsedQuery {
        fuzzy,
        tags,
        negations,
        prefix,
        exact,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_struct() {
        let q = ParsedQuery::default();
        assert_eq!(q.fuzzy, "");
        assert!(q.tags.is_empty());
    }
}
