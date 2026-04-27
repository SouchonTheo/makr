use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// The kind of variable assignment used in a Makefile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignmentKind {
    /// `=` — recursively expanded
    Recursive,
    /// `:=` or `::=` — simply expanded
    Simple,
    /// `?=` — conditional (only if not already defined)
    Conditional,
    /// `+=` — append
    Append,
}

pub struct MakeVariable {
    pub name: String,
    pub value: String,
    pub assignment: AssignmentKind,
}

pub struct MakeTarget {
    pub name: String,
    pub dependencies: Vec<String>,
    pub commands: Vec<String>,
    pub is_phony: bool,
}

/// Find the position of the target separator ':' in a line,
/// skipping ':' that appear inside $(...) or ${...} expansions.
fn find_target_colon(line: &str) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in line.char_indices() {
        match c {
            '$' => {}
            '(' | '{' if i > 0 && line.as_bytes()[i - 1] == b'$' => depth += 1,
            ')' | '}' if depth > 0 => depth -= 1,
            ':' if depth == 0 => return Some(i),
            _ => {}
        }
    }
    None
}

/// Join continuation lines (lines ending with `\`) into single logical lines.
fn join_continuation_lines(content: &str) -> Vec<String> {
    let mut logical_lines = Vec::new();
    let mut current = String::new();

    for line in content.lines() {
        if let Some(stripped) = line.strip_suffix('\\') {
            current.push_str(stripped);
            current.push(' ');
        } else {
            current.push_str(line);
            logical_lines.push(std::mem::take(&mut current));
        }
    }
    // Flush remaining content if file ends mid-continuation
    if !current.is_empty() {
        logical_lines.push(current);
    }
    logical_lines
}

/// Detect the assignment operator in a line and return (position of operator start, AssignmentKind).
/// Returns None if the line does not contain a variable assignment.
fn detect_assignment(line: &str) -> Option<(usize, AssignmentKind)> {
    let bytes = line.as_bytes();
    let mut depth = 0usize;

    for i in 0..bytes.len() {
        match bytes[i] {
            b'$' => {}
            b'(' | b'{' if i > 0 && bytes[i - 1] == b'$' => depth += 1,
            b')' | b'}' if depth > 0 => depth -= 1,
            b':' if depth == 0 => {
                // := (simple)
                if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                    return Some((i, AssignmentKind::Simple));
                }
                // ::= (simple, alternate form)
                if i + 2 < bytes.len() && bytes[i + 1] == b':' && bytes[i + 2] == b'=' {
                    return Some((i, AssignmentKind::Simple));
                }
                // A bare colon means this is a target rule, not an assignment
            }
            b'?' if depth == 0 && i + 1 < bytes.len() && bytes[i + 1] == b'=' => {
                return Some((i, AssignmentKind::Conditional));
            }
            b'+' if depth == 0 && i + 1 < bytes.len() && bytes[i + 1] == b'=' => {
                return Some((i, AssignmentKind::Append));
            }
            b'=' if depth == 0 => {
                // Plain recursive assignment (the :, ?, + cases are caught above)
                return Some((i, AssignmentKind::Recursive));
            }
            _ => {}
        }
    }
    None
}

/// Strip leading `export` and/or `override` prefixes from a line
/// to expose the underlying variable assignment.
fn strip_var_prefixes(line: &str) -> &str {
    let mut s = line;
    loop {
        let before = s;
        if let Some(rest) = s.strip_prefix("export ") {
            s = rest.trim_start();
        }
        if let Some(rest) = s.strip_prefix("override ") {
            s = rest.trim_start();
        }
        if s == before {
            break;
        }
    }
    s
}

/// Accumulated state for parsing one or more Makefile files.
struct ParseState {
    variables: Vec<MakeVariable>,
    targets: Vec<MakeTarget>,
    visited: HashSet<PathBuf>,
    phony_names: HashSet<String>,
    defined_vars: HashMap<String, usize>,
}

impl ParseState {
    fn new() -> Self {
        Self {
            variables: Vec::new(),
            targets: Vec::new(),
            visited: HashSet::new(),
            phony_names: HashSet::new(),
            defined_vars: HashMap::new(),
        }
    }

    /// Parse a Makefile at the given path, following `include` directives.
    fn parse_file(&mut self, path: &Path) -> Result<(), String> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if !self.visited.insert(canonical.clone()) {
            return Ok(()); // Already parsed, avoid cycles
        }

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                return Err(format!("Failed to read '{}': {}", path.display(), e));
            }
        };

        let base_dir = path.parent().unwrap_or(Path::new("."));
        let mut current_target: Option<MakeTarget> = None;

        let logical_lines = join_continuation_lines(&content);
        let mut line_iter = logical_lines.iter();

        while let Some(line) = line_iter.next() {
            // Skip comments and empty lines
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }

            // Command line (starts with tab)
            if line.starts_with('\t') {
                if let Some(ref mut target) = current_target {
                    target.commands.push(line.trim().to_string());
                }
                continue;
            }

            let trimmed = line.trim();

            // Skip `define ... endef` blocks
            if trimmed.starts_with("define ") || trimmed == "define" {
                for inner in line_iter.by_ref() {
                    if inner.trim() == "endef" {
                        break;
                    }
                }
                continue;
            }

            // Handle include / -include / sinclude directives
            if let Some(rest) = trimmed
                .strip_prefix("include ")
                .or_else(|| trimmed.strip_prefix("-include "))
                .or_else(|| trimmed.strip_prefix("sinclude "))
            {
                if let Some(target) = current_target.take() {
                    self.targets.push(target);
                }

                let is_optional =
                    trimmed.starts_with("-include") || trimmed.starts_with("sinclude");

                for include_path in rest.split_whitespace() {
                    let resolved = base_dir.join(include_path);
                    if include_path.contains('*') {
                        if let Ok(entries) = glob_paths(&resolved) {
                            for entry in entries {
                                let _ = self.parse_file(&entry);
                            }
                        }
                    } else {
                        match self.parse_file(&resolved) {
                            Ok(()) => {}
                            Err(_) if is_optional => {}
                            Err(e) => return Err(e),
                        }
                    }
                }
                continue;
            }

            // Strip `export` / `override` prefixes before checking for assignments
            let stripped = strip_var_prefixes(trimmed);

            // Variable assignment detection
            if let Some((op_pos, kind)) = detect_assignment(stripped) {
                let name = stripped[..op_pos].trim().to_string();
                let op_len = match kind {
                    AssignmentKind::Recursive => 1,
                    AssignmentKind::Simple => {
                        if stripped[op_pos..].starts_with("::=") {
                            3
                        } else {
                            2
                        }
                    }
                    AssignmentKind::Conditional => 2,
                    AssignmentKind::Append => 2,
                };
                let value = stripped[op_pos + op_len..].trim().to_string();

                if !name.is_empty() && !name.contains(' ') {
                    if let Some(target) = current_target.take() {
                        self.targets.push(target);
                    }

                    match kind {
                        AssignmentKind::Conditional => {
                            // ?= only assigns if not already defined
                            if !self.defined_vars.contains_key(&name) {
                                let idx = self.variables.len();
                                self.defined_vars.insert(name.clone(), idx);
                                self.variables.push(MakeVariable {
                                    name,
                                    value,
                                    assignment: kind,
                                });
                            }
                        }
                        AssignmentKind::Append => {
                            if let Some(&idx) = self.defined_vars.get(&name) {
                                self.variables[idx].value.push(' ');
                                self.variables[idx].value.push_str(&value);
                            } else {
                                let idx = self.variables.len();
                                self.defined_vars.insert(name.clone(), idx);
                                self.variables.push(MakeVariable {
                                    name,
                                    value,
                                    assignment: kind,
                                });
                            }
                        }
                        _ => {
                            // = or := : define or redefine
                            if let Some(&idx) = self.defined_vars.get(&name) {
                                self.variables[idx].value = value;
                                self.variables[idx].assignment = kind;
                            } else {
                                let idx = self.variables.len();
                                self.defined_vars.insert(name.clone(), idx);
                                self.variables.push(MakeVariable {
                                    name,
                                    value,
                                    assignment: kind,
                                });
                            }
                        }
                    }
                    continue;
                }
            }

            // .PHONY detection
            if trimmed.starts_with(".PHONY") {
                if let Some(colon_pos) = find_target_colon(trimmed) {
                    let deps_str = &trimmed[colon_pos + 1..];
                    for dep in deps_str.split_whitespace() {
                        self.phony_names.insert(dep.to_string());
                    }
                }
                continue;
            }

            // Target line
            if let Some(colon_pos) = find_target_colon(trimmed) {
                let name = trimmed[..colon_pos].trim();
                let deps_str = &trimmed[colon_pos + 1..];

                if name.is_empty() || name.starts_with('.') || name.contains('%') {
                    continue;
                }

                if let Some(target) = current_target.take() {
                    self.targets.push(target);
                }

                let dependencies: Vec<String> =
                    deps_str.split_whitespace().map(|s| s.to_string()).collect();

                current_target = Some(MakeTarget {
                    name: name.to_string(),
                    dependencies,
                    commands: Vec::new(),
                    is_phony: false,
                });
            }
        }

        if let Some(target) = current_target {
            self.targets.push(target);
        }

        Ok(())
    }
}

/// Parse a Makefile at the given path, following `include` directives.
pub fn parse_makefile(path: &Path) -> Result<(Vec<MakeVariable>, Vec<MakeTarget>), String> {
    let mut state = ParseState::new();

    state.parse_file(path)?;

    // Apply .PHONY markings to targets
    for target in &mut state.targets {
        if state.phony_names.contains(&target.name) {
            target.is_phony = true;
        }
    }

    Ok((state.variables, state.targets))
}

/// Simple glob expansion for include paths (supports `*` in filename).
fn glob_paths(pattern: &Path) -> Result<Vec<PathBuf>, ()> {
    let parent = pattern.parent().ok_or(())?;
    let file_pattern = pattern.file_name().ok_or(())?.to_str().ok_or(())?;

    let entries = fs::read_dir(parent).map_err(|_| ())?;
    let mut result: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name();
            let name = name.to_str().unwrap_or("");
            matches_glob(file_pattern, name)
        })
        .map(|e| e.path())
        .collect();
    result.sort();
    Ok(result)
}

/// Simple glob matching supporting `*` wildcard.
fn matches_glob(pattern: &str, name: &str) -> bool {
    if let Some((prefix, suffix)) = pattern.split_once('*') {
        name.starts_with(prefix)
            && name.ends_with(suffix)
            && name.len() >= prefix.len() + suffix.len()
    } else {
        pattern == name
    }
}

/// Extract all variable names referenced via `$(VAR)` or `${VAR}` in a string.
///
/// Recurses into the body of function-like constructs (e.g. `$(call f,$(VAR))`,
/// `$(wildcard $(DIR)/*.c)`) so nested references are still picked up.
fn extract_var_references(text: &str) -> Vec<String> {
    let mut vars = Vec::new();
    let mut seen = HashSet::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'$' {
            let (open, close) = match bytes[i + 1] {
                b'(' => (b'(', b')'),
                b'{' => (b'{', b'}'),
                _ => {
                    i += 1;
                    continue;
                }
            };
            let start = i + 2;
            let mut depth = 1;
            let mut end = start;
            while end < bytes.len() && depth > 0 {
                if bytes[end] == open {
                    depth += 1;
                } else if bytes[end] == close {
                    depth -= 1;
                }
                if depth > 0 {
                    end += 1;
                }
            }
            if depth == 0 {
                let name = &text[start..end];
                // Simple variable name (e.g. `$(CC)`, `$(BUILD_DIR)`).
                // Reject names that look like function calls or contain expansions —
                // but still keep scanning past `$(` so nested `$(VAR)` references inside
                // are picked up.
                if !name.is_empty()
                    && !name.contains(' ')
                    && !name.contains('\t')
                    && !name.contains('$')
                    && !name.contains(':')
                    && !name.contains(',')
                    && seen.insert(name.to_string())
                {
                    vars.push(name.to_string());
                }
            }
            // Advance past `$(` only — don't skip the body, so we still see nested
            // references like `$(SNAPSHOT_DIR)` inside `$(call f,...,$(SNAPSHOT_DIR))`.
            i += 2;
        } else {
            i += 1;
        }
    }
    vars
}

/// Find all variable names referenced via `$(VAR)` or `${VAR}` in the target's
/// commands and dependencies. Returns cloned `MakeVariable`s for defined variables
/// and synthesized entries (with empty value) for undefined ones.
pub fn find_all_used_variables(
    target: &MakeTarget,
    variables: &[MakeVariable],
) -> Vec<MakeVariable> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();

    // Collect all referenced variable names
    let all_text = target.commands.iter().chain(target.dependencies.iter());
    for text in all_text {
        for name in extract_var_references(text) {
            if seen.insert(name.clone()) {
                if let Some(var) = variables.iter().find(|v| v.name == name) {
                    result.push(MakeVariable {
                        name: var.name.clone(),
                        value: var.value.clone(),
                        assignment: var.assignment.clone(),
                    });
                } else {
                    result.push(MakeVariable {
                        name,
                        value: String::new(),
                        assignment: AssignmentKind::Recursive,
                    });
                }
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_makefiles_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_makefiles")
    }

    // ── find_target_colon ────────────────────────────────────────

    #[test]
    fn find_target_colon_simple() {
        assert_eq!(find_target_colon("all: main.o"), Some(3));
    }

    #[test]
    fn find_target_colon_with_expansion() {
        // The colon inside $(PREFIX) should be skipped; the target colon is after the closing paren
        assert_eq!(find_target_colon("$(NAME): main.o"), Some(7));
    }

    #[test]
    fn find_target_colon_no_colon() {
        assert_eq!(find_target_colon("FOO = bar"), None);
    }

    #[test]
    fn find_target_colon_simple_assign() {
        // `:=` — the `:` before `=` is still a bare colon at depth 0
        let result = find_target_colon("CC := gcc");
        assert_eq!(result, Some(3));
    }

    #[test]
    fn find_target_colon_nested_expansion() {
        // The inner colon is at depth > 0; the target colon is at index 13
        assert_eq!(find_target_colon("$($(FOO):bar): dep"), Some(13));
    }

    #[test]
    fn find_target_colon_curly_braces() {
        assert_eq!(find_target_colon("${NAME}: dep"), Some(7));
    }

    // ── matches_glob ─────────────────────────────────────────────

    #[test]
    fn matches_glob_exact() {
        assert!(matches_glob("Makefile", "Makefile"));
    }

    #[test]
    fn matches_glob_wildcard() {
        assert!(matches_glob("*.mk", "config.mk"));
        assert!(matches_glob("*.mk", "colors.mk"));
        assert!(matches_glob("test_*", "test_foo"));
    }

    #[test]
    fn matches_glob_no_match() {
        assert!(!matches_glob("*.mk", "Makefile"));
        assert!(!matches_glob("*.rs", "config.mk"));
    }

    #[test]
    fn matches_glob_star_matches_empty() {
        // "*" prefix is empty and suffix is ".mk", so ".mk" matches
        assert!(matches_glob("*.mk", ".mk"));
    }

    #[test]
    fn matches_glob_exact_no_match() {
        assert!(!matches_glob("Makefile", "makefile"));
    }

    // ── parse_makefile: simple.mk ────────────────────────────────

    #[test]
    fn parse_simple_mk_targets() {
        let path = test_makefiles_dir().join("simple.mk");
        let (variables, targets) = parse_makefile(&path).unwrap();

        assert!(variables.is_empty(), "simple.mk has no variables");
        let names: Vec<&str> = targets.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, vec!["all", "hello", "goodbye", "clean"]);
    }

    #[test]
    fn parse_simple_mk_deps() {
        let path = test_makefiles_dir().join("simple.mk");
        let (_, targets) = parse_makefile(&path).unwrap();

        let all = targets.iter().find(|t| t.name == "all").unwrap();
        assert_eq!(all.dependencies, vec!["hello", "goodbye"]);

        let hello = targets.iter().find(|t| t.name == "hello").unwrap();
        assert!(hello.dependencies.is_empty());
    }

    #[test]
    fn parse_simple_mk_commands() {
        let path = test_makefiles_dir().join("simple.mk");
        let (_, targets) = parse_makefile(&path).unwrap();

        let hello = targets.iter().find(|t| t.name == "hello").unwrap();
        assert_eq!(hello.commands, vec!["echo \"Hello, world!\""]);
    }

    // ── parse_makefile: empty.mk ─────────────────────────────────

    #[test]
    fn parse_empty_mk_no_targets() {
        let path = test_makefiles_dir().join("empty.mk");
        let (variables, targets) = parse_makefile(&path).unwrap();

        assert!(targets.is_empty(), "empty.mk should have no targets");
        assert_eq!(variables.len(), 2);
    }

    #[test]
    fn parse_empty_mk_variables() {
        let path = test_makefiles_dir().join("empty.mk");
        let (variables, _) = parse_makefile(&path).unwrap();

        let foo = variables.iter().find(|v| v.name == "FOO").unwrap();
        assert_eq!(foo.value, "bar");
        assert_eq!(foo.assignment, AssignmentKind::Recursive);

        let baz = variables.iter().find(|v| v.name == "BAZ").unwrap();
        assert_eq!(baz.value, "qux");
        assert_eq!(baz.assignment, AssignmentKind::Recursive);
    }

    // ── parse_makefile: config.mk ────────────────────────────────

    #[test]
    fn parse_config_mk_variables() {
        let path = test_makefiles_dir().join("config.mk");
        let (variables, targets) = parse_makefile(&path).unwrap();

        let var_names: Vec<&str> = variables.iter().map(|v| v.name.as_str()).collect();
        assert!(var_names.contains(&"DEBUG"));
        assert!(var_names.contains(&"OPTFLAGS"));
        assert!(var_names.contains(&"LOG_LEVEL"));

        let debug = variables.iter().find(|v| v.name == "DEBUG").unwrap();
        assert_eq!(debug.value, "1");
        assert_eq!(debug.assignment, AssignmentKind::Recursive);

        assert!(!targets.is_empty(), "config.mk has targets");
    }

    // ── parse_makefile: complex.mk ───────────────────────────────

    #[test]
    fn parse_complex_mk_assignment_kinds() {
        let path = test_makefiles_dir().join("complex.mk");
        let (variables, _) = parse_makefile(&path).unwrap();

        let cc = variables.iter().find(|v| v.name == "CC").unwrap();
        assert_eq!(cc.value, "clang");
        assert_eq!(cc.assignment, AssignmentKind::Simple);

        let cflags = variables.iter().find(|v| v.name == "CFLAGS").unwrap();
        assert_eq!(cflags.assignment, AssignmentKind::Conditional);
        assert_eq!(cflags.value, "-Wall -Wextra -Werror");

        let ldflags = variables.iter().find(|v| v.name == "LDFLAGS").unwrap();
        assert_eq!(ldflags.assignment, AssignmentKind::Append);
        assert_eq!(ldflags.value, "-lm -lpthread");

        let prefix = variables.iter().find(|v| v.name == "PREFIX").unwrap();
        assert_eq!(prefix.assignment, AssignmentKind::Recursive);
        assert_eq!(prefix.value, "/usr/local");
    }

    #[test]
    fn parse_complex_mk_phony_targets() {
        let path = test_makefiles_dir().join("complex.mk");
        let (_, targets) = parse_makefile(&path).unwrap();

        let phony_expected = [
            "all",
            "clean",
            "test",
            "install",
            "uninstall",
            "dist",
            "docker",
        ];
        for name in &phony_expected {
            let t = targets
                .iter()
                .find(|t| t.name == *name)
                .unwrap_or_else(|| panic!("target '{}' not found", name));
            assert!(t.is_phony, "expected '{}' to be phony", name);
        }

        // "lint" is NOT in the .PHONY line
        let lint = targets.iter().find(|t| t.name == "lint").unwrap();
        assert!(!lint.is_phony, "lint should not be phony");
    }

    #[test]
    fn parse_complex_mk_has_many_targets() {
        let path = test_makefiles_dir().join("complex.mk");
        let (_, targets) = parse_makefile(&path).unwrap();

        let names: Vec<&str> = targets.iter().map(|t| t.name.as_str()).collect();
        let expected = [
            "all",
            "clean",
            "test",
            "install",
            "uninstall",
            "dist",
            "lint",
            "format",
            "docker",
            "docker-push",
            "benchmark",
            "docs",
        ];
        for name in &expected {
            assert!(names.contains(name), "missing target '{}'", name);
        }
    }

    // ── parse_makefile: error case ───────────────────────────────

    #[test]
    fn parse_makefile_nonexistent_returns_err() {
        let result = parse_makefile(Path::new("/nonexistent/Makefile"));
        assert!(result.is_err());
    }

    // ── parse_makefile: multi_phony.mk ────────────────────────────

    #[test]
    fn parse_multi_phony_mk() {
        let path = test_makefiles_dir().join("multi_phony.mk");
        let (_, targets) = parse_makefile(&path).unwrap();

        let build = targets.iter().find(|t| t.name == "build").unwrap();
        assert!(build.is_phony, "build should be phony");

        let test = targets.iter().find(|t| t.name == "test").unwrap();
        assert!(test.is_phony, "test should be phony");

        let clean = targets.iter().find(|t| t.name == "clean").unwrap();
        assert!(clean.is_phony, "clean should be phony");

        let install = targets.iter().find(|t| t.name == "install").unwrap();
        assert!(!install.is_phony, "install should NOT be phony");
    }

    // ── extract_var_references ──────────────────────────────────

    #[test]
    fn extract_var_refs_paren() {
        let refs = extract_var_references("$(CC) -o $(NAME)");
        assert_eq!(refs, vec!["CC", "NAME"]);
    }

    #[test]
    fn extract_var_refs_curly() {
        let refs = extract_var_references("${CC} -o ${NAME}");
        assert_eq!(refs, vec!["CC", "NAME"]);
    }

    #[test]
    fn extract_var_refs_mixed() {
        let refs = extract_var_references("$(CC) ${NAME}");
        assert_eq!(refs, vec!["CC", "NAME"]);
    }

    #[test]
    fn extract_var_refs_skips_functions() {
        // $(wildcard src/*.c) has a space — should be skipped
        let refs = extract_var_references("$(wildcard src/*.c) $(CC)");
        assert_eq!(refs, vec!["CC"]);
    }

    #[test]
    fn extract_var_refs_nested_indirect() {
        // $($(FOO)) — the outer expansion isn't a simple name, but FOO is read
        // to compute the inner name, so FOO should still be reported.
        let refs = extract_var_references("$($(FOO))");
        assert_eq!(refs, vec!["FOO"]);
    }

    #[test]
    fn extract_var_refs_inside_call() {
        // $(call run-dylint,tfhe_lints_snapshot,$(SNAPSHOT_DIR)) — the outer
        // $(call ...) is a function invocation, but $(SNAPSHOT_DIR) inside is a
        // real variable reference and must be picked up.
        let refs = extract_var_references(
            "$(call run-dylint,tfhe_lints_snapshot,$(SNAPSHOT_DIR)) $(call crate-args,$(CRATE))",
        );
        assert!(refs.contains(&"SNAPSHOT_DIR".to_string()), "got {:?}", refs);
        assert!(refs.contains(&"CRATE".to_string()), "got {:?}", refs);
    }

    #[test]
    fn extract_var_refs_inside_wildcard() {
        // Variable references nested inside a function call should be found.
        let refs = extract_var_references("$(wildcard $(SRC_DIR)/*.c)");
        assert_eq!(refs, vec!["SRC_DIR"]);
    }

    #[test]
    fn extract_var_refs_no_duplicates() {
        let refs = extract_var_references("$(CC) $(CC) $(NAME)");
        assert_eq!(refs, vec!["CC", "NAME"]);
    }

    #[test]
    fn extract_var_refs_none() {
        let refs = extract_var_references("echo hello");
        assert!(refs.is_empty());
    }

    // ── find_all_used_variables ─────────────────────────────────

    #[test]
    fn find_all_none_used() {
        let vars = vec![MakeVariable {
            name: "FOO".into(),
            value: "bar".into(),
            assignment: AssignmentKind::Recursive,
        }];
        let target = MakeTarget {
            name: "clean".into(),
            dependencies: vec![],
            commands: vec!["rm -f *.o".into()],
            is_phony: false,
        };

        let all = find_all_used_variables(&target, &vars);
        assert!(all.is_empty());
    }

    #[test]
    fn find_all_curly_brace_syntax() {
        let vars = vec![MakeVariable {
            name: "CC".into(),
            value: "gcc".into(),
            assignment: AssignmentKind::Recursive,
        }];
        let target = MakeTarget {
            name: "build".into(),
            dependencies: vec![],
            commands: vec!["${CC} -o out main.c".into()],
            is_phony: false,
        };

        let all = find_all_used_variables(&target, &vars);
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "CC");
    }

    #[test]
    fn find_all_includes_undefined_vars() {
        let vars = vec![MakeVariable {
            name: "CC".into(),
            value: "gcc".into(),
            assignment: AssignmentKind::Recursive,
        }];
        let target = MakeTarget {
            name: "build".into(),
            dependencies: vec![],
            commands: vec!["$(CC) -o $(NAME) main.c".into()],
            is_phony: false,
        };

        let all = find_all_used_variables(&target, &vars);
        assert_eq!(all.len(), 2);

        let cc = all.iter().find(|v| v.name == "CC").unwrap();
        assert_eq!(cc.value, "gcc");

        let name = all.iter().find(|v| v.name == "NAME").unwrap();
        assert_eq!(name.value, "", "undefined var should have empty value");
    }

    #[test]
    fn find_all_no_duplicates() {
        let vars = vec![];
        let target = MakeTarget {
            name: "build".into(),
            dependencies: vec!["$(NAME)".into()],
            commands: vec!["echo $(NAME)".into()],
            is_phony: false,
        };

        let all = find_all_used_variables(&target, &vars);
        assert_eq!(all.len(), 1, "NAME should appear only once");
        assert_eq!(all[0].name, "NAME");
    }

    #[test]
    fn find_all_from_dependencies() {
        let vars = vec![];
        let target = MakeTarget {
            name: "run".into(),
            dependencies: vec!["$(BUILD_DIR)/app".into()],
            commands: vec![],
            is_phony: false,
        };

        let all = find_all_used_variables(&target, &vars);
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "BUILD_DIR");
    }

    // ── find_all_used_variables with includes ───────────────────

    #[test]
    fn find_all_resolves_vars_from_includes() {
        let path = test_makefiles_dir().join("Makefile");
        let (variables, targets) = parse_makefile(&path).unwrap();

        // "debug" target uses RED, LOG_LEVEL, RESET (from includes) and NAME (from Makefile)
        let debug = targets.iter().find(|t| t.name == "debug").unwrap();
        let all = find_all_used_variables(debug, &variables);

        let names: Vec<&str> = all.iter().map(|v| v.name.as_str()).collect();
        assert!(
            names.contains(&"RED"),
            "RED (from colors.mk) missing: {:?}",
            names
        );
        assert!(
            names.contains(&"LOG_LEVEL"),
            "LOG_LEVEL (from config.mk) missing: {:?}",
            names
        );
        assert!(
            names.contains(&"RESET"),
            "RESET (from colors.mk) missing: {:?}",
            names
        );
        assert!(
            names.contains(&"NAME"),
            "NAME (from Makefile) missing: {:?}",
            names
        );

        // They should have their values from the included files
        let red = all.iter().find(|v| v.name == "RED").unwrap();
        assert!(
            !red.value.is_empty(),
            "RED should have a value from colors.mk"
        );

        let log_level = all.iter().find(|v| v.name == "LOG_LEVEL").unwrap();
        assert_eq!(log_level.value, "verbose");
    }
}
