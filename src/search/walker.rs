use camino::Utf8PathBuf;
use ignore::WalkBuilder;

/// A single directory entry returned by the walker.
pub struct PathEntry {
    /// The full UTF-8 path to the directory.
    pub path: Utf8PathBuf,
    /// The final component of the path (the directory name).
    pub name: String,
    /// A human-readable display string for the parent path.
    pub parent_display: String,
}

/// Options controlling how the directory walk is performed.
pub struct WalkOptions {
    /// Maximum depth to recurse (1 = immediate children only).
    pub max_depth: usize,
    /// Whether to include hidden entries (those whose name starts with `.`).
    pub show_hidden: bool,
    /// Whether to honour `.gitignore` / `.ignore` files.
    pub respect_gitignore: bool,
    /// Whether to follow symbolic links.
    pub follow_symlinks: bool,
}

/// Walk each root in `roots` and collect all directories found, honouring `opts`.
///
/// The root directory itself is **not** included in the output.
/// Entries whose path contains non-UTF-8 bytes are silently skipped.
pub fn walk_directories(roots: &[String], opts: &WalkOptions) -> Vec<PathEntry> {
    let mut entries: Vec<PathEntry> = Vec::new();

    for root in roots {
        // Build the walker for this root.
        let mut builder = WalkBuilder::new(root);
        builder
            .max_depth(Some(opts.max_depth))
            .hidden(!opts.show_hidden)
            .git_ignore(opts.respect_gitignore)
            .git_global(opts.respect_gitignore)
            .git_exclude(opts.respect_gitignore)
            .follow_links(opts.follow_symlinks);

        for result in builder.build() {
            let dir_entry = match result {
                Ok(e) => e,
                Err(_) => continue,
            };

            // Skip the root itself (depth 0).
            if dir_entry.depth() == 0 {
                continue;
            }

            // We only want directories.
            let file_type = match dir_entry.file_type() {
                Some(ft) => ft,
                None => continue,
            };
            if !file_type.is_dir() {
                continue;
            }

            // Require a valid UTF-8 path.
            let path_str = match dir_entry.path().to_str() {
                Some(s) => s,
                None => continue,
            };
            let utf8_path = Utf8PathBuf::from(path_str);

            let name = utf8_path
                .file_name()
                .unwrap_or("")
                .to_string();

            let parent_display = utf8_path
                .parent()
                .map(|p| p.as_str().to_string())
                .unwrap_or_default();

            entries.push(PathEntry {
                path: utf8_path,
                name,
                parent_display,
            });
        }
    }

    entries
}
