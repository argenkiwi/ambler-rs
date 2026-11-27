//! The structural conventions of an Ambler project, checked against its files.
//!
//! Each rule has an id, explained in `ambler help check`.

use super::register::declared;
use super::source::{has_impl, impls, parse, use_paths};
use crate::scaffold::lint::{Violation, spec_nodes, spec_sections};
use crate::scaffold::names;
use std::collections::BTreeMap;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{ImplItem, Item, ItemTrait, TraitItem, Visibility};

const GROUPS: [&str; 3] = ["nodes", "walks", "utils"];

/// The one module that declares the state traits.
const REGISTRY: &str = "src/nodes/mod.rs";

/// The field a state trait named `name` covers: `HasExitCode` → `exit_code`. `None` if `name` is
/// not `Has` followed by an uppercase letter (`Hash` is not a state trait).
fn state_field(name: &str) -> Option<String> {
    let suffix = name.strip_prefix("Has")?;
    suffix
        .starts_with(|c: char| c.is_ascii_uppercase())
        .then(|| names::snake(suffix))
}

/// Finds the state traits (`trait Has*`) declared anywhere in a file.
#[derive(Default)]
struct StateTraits(Vec<ItemTrait>);

impl<'ast> Visit<'ast> for StateTraits {
    fn visit_item_trait(&mut self, item: &'ast ItemTrait) {
        if state_field(&item.ident.to_string()).is_some() {
            self.0.push(item.clone());
        }
        syn::visit::visit_item_trait(self, item);
    }
}

struct Linter<'a> {
    files: &'a BTreeMap<String, String>,
    found: Vec<Violation>,
}

/// The module name of `path` if it is a file directly in `src/<group>/` other than `mod.rs`.
fn member<'a>(path: &'a str, group: &str) -> Option<&'a str> {
    path.strip_prefix("src/")?
        .strip_prefix(group)?
        .strip_prefix('/')?
        .strip_suffix(".rs")
        .filter(|stem| *stem != "mod" && !stem.contains('/'))
}

fn has_tests(file: &syn::File) -> bool {
    file.items
        .iter()
        .any(|item| matches!(item, Item::Mod(item) if item.ident == "tests"))
}

/// Finds the `_ =>` arms in a walk's `step`.
#[derive(Default)]
struct Wildcards(Vec<usize>);

impl<'ast> Visit<'ast> for Wildcards {
    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        if matches!(arm.pat, syn::Pat::Wild(_)) {
            self.0.push(arm.pat.span().start().line);
        }
        syn::visit::visit_arm(self, arm);
    }
}

impl Linter<'_> {
    fn report(&mut self, path: &str, line: usize, rule: &'static str, message: String) {
        self.found.push(Violation {
            path: path.to_string(),
            line,
            rule,
            message,
        });
    }

    /// Every file in a group is declared in its `mod.rs`, in alphabetical order.
    fn modules(&mut self, group: &str) {
        let path = format!("src/{group}/mod.rs");
        let Some(source) = self.files.get(&path) else {
            self.report(
                &path,
                1,
                "layout",
                format!("missing: an Ambler project keeps its {group} in src/{group}/"),
            );
            return;
        };
        let declarations: Vec<(usize, &str)> = source
            .lines()
            .enumerate()
            .filter_map(|(index, line)| Some((index + 1, declared(line)?)))
            .collect();
        for pair in declarations.windows(2) {
            let ((_, previous), (line, next)) = (pair[0], pair[1]);
            if next < previous {
                self.report(
                    &path,
                    line,
                    "sorted",
                    format!("`pub mod {next};` belongs before `pub mod {previous};`"),
                );
            }
        }
        let unregistered: Vec<&String> = self
            .files
            .keys()
            .filter(|file| {
                member(file, group)
                    .is_some_and(|stem| declarations.iter().all(|(_, name)| *name != stem))
            })
            .collect();
        for file in unregistered {
            let stem = member(file, group).unwrap_or_default();
            self.report(
                file,
                1,
                "registered",
                format!("add `pub mod {stem};` to {path}"),
            );
        }
    }

    /// Parses a file, reporting it if it is not valid Rust, and flags what is left unfinished.
    fn parsed(&mut self, path: &str, source: &str) -> Option<syn::File> {
        for (index, line) in source.lines().enumerate() {
            if line.contains("todo!(") {
                self.report(path, index + 1, "todo", "still to be written".to_string());
            }
        }
        match parse(source) {
            Ok(file) => Some(file),
            Err((line, error)) => {
                self.report(path, line, "parse", error);
                None
            }
        }
    }

    fn node(&mut self, path: &str, stem: &str, source: &str) {
        let Some(file) = self.parsed(path, source) else {
            return;
        };
        let pascal = names::pascal(stem);
        let has = |wanted: &str| {
            file.items.iter().any(|item| match item {
                Item::Struct(item) => item.ident == wanted,
                Item::Trait(item) => item.ident == wanted,
                _ => false,
            })
        };
        if !has(&pascal) {
            self.report(
                path,
                1,
                "node-shape",
                format!("expected a struct `{pascal}`, named after the file"),
            );
        }
        if !has_impl(&file, "Node") {
            self.report(path, 1, "node-shape", "expected an `impl Node`".to_string());
        }
        if has("Utils") && !has("SystemUtils") {
            self.report(
                path,
                1,
                "node-shape",
                "`Utils` needs its production implementation, `SystemUtils`".to_string(),
            );
        }
        if !has_tests(&file) {
            self.report(
                path,
                1,
                "node-tests",
                "expected an inline `#[cfg(test)] mod tests`, one test per branch".to_string(),
            );
        }
    }

    fn walk(&mut self, path: &str, stem: &str, source: &str) {
        let Some(file) = self.parsed(path, source) else {
            return;
        };
        for item in &file.items {
            let (visibility, name) = match item {
                Item::Fn(item) => (&item.vis, item.sig.ident.to_string()),
                Item::Struct(item) => (&item.vis, item.ident.to_string()),
                Item::Enum(item) => (&item.vis, item.ident.to_string()),
                Item::Type(item) => (&item.vis, item.ident.to_string()),
                Item::Trait(item) => (&item.vis, item.ident.to_string()),
                Item::Const(item) => (&item.vis, item.ident.to_string()),
                Item::Static(item) => (&item.vis, item.ident.to_string()),
                Item::Mod(item) => (&item.vis, item.ident.to_string()),
                Item::Use(item) => (&item.vis, "use".to_string()),
                _ => continue,
            };
            let allowed = match item {
                Item::Fn(_) => name == "run",
                Item::Struct(_) | Item::Enum(_) | Item::Type(_) => name == "Error",
                _ => false,
            };
            if !matches!(visibility, Visibility::Inherited) && !allowed {
                self.report(
                    path,
                    item.span().start().line,
                    "walk-surface",
                    format!("`{name}` should be private: a walk exports only `Error` and `run`"),
                );
            }
        }
        if !has_tests(&file) {
            self.report(
                path,
                1,
                "walk-tests",
                "expected an inline `#[cfg(test)] mod tests` that tests the wiring".to_string(),
            );
        }

        let mut wildcards = Wildcards::default();
        for item in impls(&file, "Walk").flat_map(|item| &item.items) {
            if let ImplItem::Fn(method) = item {
                if method.sig.ident == "step" {
                    wildcards.visit_impl_item_fn(method);
                }
            }
        }
        for line in wildcards.0 {
            self.report(
                path,
                line,
                "wildcard-arm",
                "match every outcome by name, so a new one is a compile error".to_string(),
            );
        }

        let call = format!("walks::{stem}::");
        let has_runner = self.files.iter().any(|(file, source)| {
            (file == "src/main.rs" || file.starts_with("src/bin/")) && source.contains(&call)
        });
        if !has_runner {
            self.report(
                path,
                1,
                "walk-runner",
                format!("nothing in src/main.rs or src/bin/ runs `{call}run`"),
            );
        }

        let ids: Option<Vec<String>> = file.items.iter().find_map(|item| match item {
            Item::Enum(item) if item.ident == "Id" => Some(
                item.variants
                    .iter()
                    .map(|variant| variant.ident.to_string())
                    .collect(),
            ),
            _ => None,
        });
        let spec = format!("specs/{stem}.md");
        if !self.files.contains_key(&spec) {
            self.report(path, 1, "walk-spec", format!("expected its spec in {spec}"));
        } else if let Some(ids) = ids {
            let show: fn(&str) -> String = |id| format!("Id::{id}");
            self.found
                .extend(spec_nodes(self.files, &spec, path, &ids, show));
        }
    }

    fn util(&mut self, path: &str, source: &str) {
        let Some(file) = self.parsed(path, source) else {
            return;
        };
        for item in &file.items {
            let Item::Use(item) = item else {
                continue;
            };
            for import in use_paths(item) {
                let reaches_up = [
                    "crate::nodes",
                    "crate::walks",
                    "crate::engine",
                    "crate::Node",
                ]
                .iter()
                .any(|above| import.path.starts_with(above));
                if reaches_up {
                    self.report(
                        path,
                        item.span().start().line,
                        "util-imports",
                        format!(
                            "`{}`: utilities know nothing of nodes or walks",
                            import.path
                        ),
                    );
                }
            }
        }
    }

    /// State traits are declared only in the registry, and each covers exactly one field: a
    /// getter named after it and, optionally, its `_mut` accessor.
    fn state_traits(&mut self) {
        for (path, source) in self.files {
            if !(path.starts_with("src/") && path.ends_with(".rs")) {
                continue;
            }
            // A file that does not parse is reported as `parse` where it matters.
            let Ok(file) = parse(source) else {
                continue;
            };
            let mut found = StateTraits::default();
            found.visit_file(&file);
            for item in found.0 {
                let name = item.ident.to_string();
                let line = item.span().start().line;
                if path != REGISTRY {
                    self.report(
                        path,
                        line,
                        "state-traits",
                        format!("move `{name}` to {REGISTRY}: state traits live only there"),
                    );
                    continue;
                }
                let field = state_field(&name).unwrap_or_default();
                let accessor = format!("{field}_mut");
                let mut has_getter = false;
                for member in &item.items {
                    let extra = match member {
                        TraitItem::Fn(method) if method.sig.ident == field => {
                            has_getter = true;
                            continue;
                        }
                        TraitItem::Fn(method) if method.sig.ident == accessor => continue,
                        TraitItem::Fn(method) => format!("`fn {}`", method.sig.ident),
                        _ => "anything but methods".to_string(),
                    };
                    self.report(
                        path,
                        member.span().start().line,
                        "state-traits",
                        format!(
                            "`{name}` covers one field: `{field}` and optionally `{accessor}`, not {extra}"
                        ),
                    );
                }
                if !has_getter {
                    self.report(
                        path,
                        line,
                        "state-traits",
                        format!("`{name}` needs a getter, `fn {field}(&self)`"),
                    );
                }
            }
        }
    }
}

/// Checks the conventions over `files`: paths relative to the project root, with contents.
///
/// Expects `Cargo.toml`, every `.rs` file under `src/` and every `.md` file under `specs/`.
#[must_use]
pub fn lint(files: &BTreeMap<String, String>) -> Vec<Violation> {
    let mut linter = Linter {
        files,
        found: Vec::new(),
    };
    if !files.contains_key("Cargo.toml") {
        linter.report(
            "Cargo.toml",
            1,
            "layout",
            "missing: run `cargo ambler check` from the root of an Ambler project".to_string(),
        );
        return linter.found;
    }
    for group in GROUPS {
        linter.modules(group);
    }
    linter.state_traits();
    for (path, source) in files {
        if let Some(stem) = member(path, "nodes") {
            linter.node(path, stem, source);
        } else if let Some(stem) = member(path, "walks") {
            linter.walk(path, stem, source);
        } else if member(path, "utils").is_some() {
            linter.util(path, source);
        } else if path.starts_with("specs/") && path.ends_with(".md") {
            linter.found.extend(spec_sections(files, path));
        }
    }
    linter.found.sort();
    linter.found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scaffold::rust::project::fixture;

    /// The example project, as `ambler new app --example` writes it.
    fn example() -> BTreeMap<String, String> {
        fixture(true)
    }

    /// Each violation as `(path, line, rule)`.
    fn rules(files: &BTreeMap<String, String>) -> Vec<(String, usize, &'static str)> {
        lint(files)
            .into_iter()
            .map(|violation| (violation.path, violation.line, violation.rule))
            .collect()
    }

    fn at(path: &str, line: usize, rule: &'static str) -> (String, usize, &'static str) {
        (path.to_string(), line, rule)
    }

    fn edit(files: &mut BTreeMap<String, String>, path: &str, from: &str, to: &str) {
        let source = files.get_mut(path).unwrap();
        assert!(source.contains(from), "{path} does not contain {from:?}");
        *source = source.replace(from, to);
    }

    #[test]
    fn the_example_project_follows_every_convention() {
        assert_eq!(lint(&example()), []);
    }

    #[test]
    fn an_empty_project_follows_every_convention() {
        let files = fixture(false);

        assert_eq!(lint(&files), []);
    }

    #[test]
    fn outside_a_project_there_is_nothing_to_check() {
        let violations = lint(&BTreeMap::new());

        assert_eq!(violations.len(), 1);
        assert_eq!(
            violations[0].to_string(),
            "Cargo.toml:1: layout: missing: run `cargo ambler check` from the root of an Ambler project"
        );
    }

    #[test]
    fn modules_are_registered_in_order() {
        let mut files = example();
        edit(
            &mut files,
            "src/nodes/mod.rs",
            "pub mod count;\npub mod start;\n",
            "pub mod start;\npub mod count;\n",
        );
        files.insert("src/utils/clock.rs".to_string(), "//! Time.\n".to_string());

        assert_eq!(
            rules(&files),
            [
                at("src/nodes/mod.rs", 4, "sorted"),
                at("src/utils/clock.rs", 1, "registered"),
            ]
        );
    }

    #[test]
    fn a_node_is_a_struct_named_after_its_file_with_utils_and_tests() {
        let mut files = example();
        edit(
            &mut files,
            "src/nodes/stop.rs",
            "pub struct SystemUtils;",
            "pub struct RealUtils;",
        );
        edit(
            &mut files,
            "src/nodes/stop.rs",
            "impl Utils for SystemUtils",
            "impl Utils for RealUtils",
        );
        edit(
            &mut files,
            "src/nodes/stop.rs",
            "Stop<U = SystemUtils>",
            "Stop<U = RealUtils>",
        );
        let untested = files["src/nodes/count.rs"]
            .split("#[cfg(test)]")
            .next()
            .unwrap()
            .to_string();
        files.insert(
            "src/nodes/count.rs".to_string(),
            untested.replace("Count", "Counting"),
        );

        assert_eq!(
            rules(&files),
            [
                at("src/nodes/count.rs", 1, "node-shape"),
                at("src/nodes/count.rs", 1, "node-tests"),
                at("src/nodes/stop.rs", 1, "node-shape"),
            ]
        );
    }

    #[test]
    fn a_walk_exports_error_and_run_and_matches_every_outcome_by_name() {
        let mut files = example();
        edit(
            &mut files,
            "src/walks/counter.rs",
            "\nenum Id {",
            "\npub enum Id {",
        );
        edit(
            &mut files,
            "src/walks/counter.rs",
            "count::Outcome::Finished => Transition::Next(Id::Stop),",
            "_ => Transition::Next(Id::Stop),",
        );

        assert_eq!(
            rules(&files),
            [
                at("src/walks/counter.rs", 28, "walk-surface"),
                at("src/walks/counter.rs", 69, "wildcard-arm"),
            ]
        );
    }

    #[test]
    fn a_walk_has_a_runner_and_a_spec_that_names_its_nodes() {
        let mut files = example();
        files.remove("src/main.rs");
        edit(&mut files, "specs/counter.md", "### Stop", "### Finish");
        edit(
            &mut files,
            "specs/counter.md",
            "## Shared State",
            "## State",
        );

        assert_eq!(
            rules(&files),
            [
                at("specs/counter.md", 1, "spec-nodes"),
                at("specs/counter.md", 1, "spec-sections"),
                at("specs/counter.md", 18, "spec-nodes"),
                at("specs/counter.md", 21, "spec-sections"),
                at("src/walks/counter.rs", 1, "walk-runner"),
            ]
        );

        files.remove("specs/counter.md");
        assert_eq!(
            rules(&files)[0],
            at("src/walks/counter.rs", 1, "walk-runner")
        );
        assert_eq!(rules(&files)[1], at("src/walks/counter.rs", 1, "walk-spec"));
    }

    /// Appends `source` to the file at `path`, returning the line it starts on.
    fn append(files: &mut BTreeMap<String, String>, path: &str, source: &str) -> usize {
        let file = files.get_mut(path).unwrap();
        let line = file.lines().count() + 1;
        file.push_str(source);
        line
    }

    #[test]
    fn a_state_trait_may_be_read_only_or_have_a_mut_accessor() {
        let mut files = example();
        append(
            &mut files,
            REGISTRY,
            "\npub trait HasLimit {\n    fn limit(&self) -> u32;\n}\n\
             \npub trait HasLastName {\n    fn last_name(&self) -> &str;\n    fn last_name_mut(&mut self) -> &mut String;\n}\n",
        );

        assert_eq!(lint(&files), []);
    }

    #[test]
    fn a_state_trait_covers_one_field() {
        let mut files = example();
        let line = append(
            &mut files,
            REGISTRY,
            "\npub trait HasServer {\n    fn host(&self) -> &str;\n    fn port(&self) -> u16;\n}\n",
        ) + 1;

        assert_eq!(
            rules(&files),
            [
                at(REGISTRY, line, "state-traits"),
                at(REGISTRY, line + 1, "state-traits"),
                at(REGISTRY, line + 2, "state-traits"),
            ]
        );
        let messages: Vec<String> = lint(&files).iter().map(ToString::to_string).collect();
        assert!(messages[0].contains("needs a getter, `fn server(&self)`"));
        assert!(messages[1].contains("not `fn host`"));
    }

    #[test]
    fn a_state_trait_has_no_other_methods() {
        let mut files = example();
        let line = append(
            &mut files,
            REGISTRY,
            "\npub trait HasName {\n    fn name(&self) -> &str;\n    fn name_mut(&mut self) -> &mut String;\n    fn rename(&mut self, to: &str);\n}\n",
        ) + 1;

        let violations = lint(&files);
        assert_eq!(violations.len(), 1);
        assert_eq!(
            violations[0].to_string(),
            format!(
                "{REGISTRY}:{}: state-traits: `HasName` covers one field: `name` and optionally `name_mut`, not `fn rename`",
                line + 3
            )
        );
    }

    #[test]
    fn state_traits_are_declared_only_in_the_registry() {
        let mut files = example();
        let line = append(
            &mut files,
            "src/nodes/stop.rs",
            "\npub trait HasLimit {\n    fn limit(&self) -> u32;\n}\n",
        ) + 1;
        append(
            &mut files,
            "src/nodes/start.rs",
            "\npub trait Hash {\n    fn hash(&self) -> u64;\n}\n",
        );

        assert_eq!(
            rules(&files),
            [at("src/nodes/stop.rs", line, "state-traits")]
        );
    }

    #[test]
    fn unfinished_code_broken_code_and_utils_reaching_up_are_reported() {
        let mut files = example();
        edit(
            &mut files,
            "src/nodes/stop.rs",
            "Ok(())\n    }\n}\n\n#[cfg",
            "todo!()\n    }\n}\n\n#[cfg",
        );
        edit(
            &mut files,
            "src/nodes/start.rs",
            "pub trait Utils {",
            "pub trait Utils {{",
        );
        edit(
            &mut files,
            "src/utils/terminal.rs",
            "use std::io::{self, Write};",
            "use crate::nodes::HasCount;\nuse std::io::{self, Write};",
        );

        let found: Vec<&str> = rules(&files).iter().map(|(_, _, rule)| *rule).collect();
        assert_eq!(found, ["parse", "todo", "util-imports"]);
    }
}
