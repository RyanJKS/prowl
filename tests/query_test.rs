use prowl::search::query::parse_query;

#[test]
fn test_plain_fuzzy_query() {
    let q = parse_query("axum api");
    assert_eq!(q.fuzzy, "axum api");
    assert!(q.tags.is_empty());
    assert!(q.negations.is_empty());
    assert!(q.prefix.is_none());
    assert!(q.exact.is_empty());
}

#[test]
fn test_tag_extraction() {
    let q = parse_query("myproject #rust #git");
    assert_eq!(q.fuzzy, "myproject");
    assert_eq!(q.tags, vec!["rust", "git"]);
}

#[test]
fn test_negation_extraction() {
    let q = parse_query("api !vendor !node_modules");
    assert_eq!(q.fuzzy, "api");
    assert_eq!(q.negations, vec!["vendor", "node_modules"]);
}

#[test]
fn test_prefix_extraction() {
    let q = parse_query("axum ^~/dev/rust");
    assert_eq!(q.fuzzy, "axum");
    assert_eq!(q.prefix, Some("~/dev/rust".to_string()));
}

#[test]
fn test_exact_match_extraction() {
    let q = parse_query("'README 'CHANGELOG");
    assert_eq!(q.fuzzy, "");
    assert_eq!(q.exact, vec!["README", "CHANGELOG"]);
}

#[test]
fn test_combined_query() {
    let q = parse_query("axum #rust !vendor ^~/dev 'README");
    assert_eq!(q.fuzzy, "axum");
    assert_eq!(q.tags, vec!["rust"]);
    assert_eq!(q.negations, vec!["vendor"]);
    assert_eq!(q.prefix, Some("~/dev".to_string()));
    assert_eq!(q.exact, vec!["README"]);
}

#[test]
fn test_escaped_special_characters() {
    let q = parse_query(r"\#my-project \!important");
    assert_eq!(q.fuzzy, "#my-project !important");
    assert!(q.tags.is_empty());
    assert!(q.negations.is_empty());
}

#[test]
fn test_empty_query() {
    let q = parse_query("");
    assert_eq!(q.fuzzy, "");
    assert!(q.tags.is_empty());
    assert!(q.negations.is_empty());
    assert!(q.prefix.is_none());
    assert!(q.exact.is_empty());
}

#[test]
fn test_only_prefix_uses_first_occurrence() {
    let q = parse_query("^~/dev ^~/work");
    assert_eq!(q.prefix, Some("~/dev".to_string()));
    assert_eq!(q.fuzzy, "^~/work");
}
