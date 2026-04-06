use prowl::search::walker::{walk_directories, WalkOptions, PathEntry};
use tempfile::TempDir;
use std::fs;

fn setup_test_dirs() -> TempDir {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path();
    fs::create_dir_all(base.join("project-a/src")).unwrap();
    fs::write(base.join("project-a/Cargo.toml"), "").unwrap();
    fs::create_dir_all(base.join("project-b/.git")).unwrap();
    fs::write(base.join("project-b/package.json"), "{}").unwrap();
    fs::create_dir_all(base.join(".hidden-dir")).unwrap();
    fs::write(base.join("file.txt"), "hello").unwrap();
    tmp
}

#[test]
fn test_walk_finds_directories() {
    let tmp = setup_test_dirs();
    let root = tmp.path().to_str().unwrap();
    let opts = WalkOptions { max_depth: 5, show_hidden: false, respect_gitignore: true, follow_symlinks: false };
    let entries = walk_directories(&[root.into()], &opts);
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"project-a"));
    assert!(names.contains(&"project-b"));
    assert!(names.contains(&"src"));
    assert!(!names.contains(&".hidden-dir"));
}

#[test]
fn test_walk_respects_depth() {
    let tmp = setup_test_dirs();
    let root = tmp.path().to_str().unwrap();
    let opts = WalkOptions { max_depth: 1, show_hidden: false, respect_gitignore: true, follow_symlinks: false };
    let entries = walk_directories(&[root.into()], &opts);
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"project-a"));
    assert!(names.contains(&"project-b"));
    assert!(!names.contains(&"src"));
}

#[test]
fn test_walk_shows_hidden_when_enabled() {
    let tmp = setup_test_dirs();
    let root = tmp.path().to_str().unwrap();
    let opts = WalkOptions { max_depth: 5, show_hidden: true, respect_gitignore: true, follow_symlinks: false };
    let entries = walk_directories(&[root.into()], &opts);
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&".hidden-dir"));
}

#[test]
fn test_path_entry_has_correct_fields() {
    let tmp = setup_test_dirs();
    let root = tmp.path().to_str().unwrap();
    let opts = WalkOptions { max_depth: 5, show_hidden: false, respect_gitignore: true, follow_symlinks: false };
    let entries = walk_directories(&[root.into()], &opts);
    let project_a = entries.iter().find(|e| e.name == "project-a").unwrap();
    assert!(project_a.path.as_str().ends_with("project-a"));
    assert!(!project_a.name.is_empty());
}

#[test]
fn test_walk_empty_root_returns_empty() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("nonexistent");
    let opts = WalkOptions { max_depth: 5, show_hidden: false, respect_gitignore: true, follow_symlinks: false };
    let entries = walk_directories(&[root.to_str().unwrap().into()], &opts);
    assert!(entries.is_empty());
}
