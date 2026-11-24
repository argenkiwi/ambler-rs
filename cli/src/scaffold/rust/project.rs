//! Reading a Rust project's manifest.

/// The name code uses for the crate a `Cargo.toml` declares: the package name with `-` as `_`.
#[must_use]
pub fn crate_name(cargo_toml: &str) -> Option<String> {
    cargo_toml
        .lines()
        .skip_while(|line| line.trim() != "[package]")
        .skip(1)
        .take_while(|line| !line.trim_start().starts_with('['))
        .find_map(|line| {
            let (key, value) = line.split_once('=')?;
            (key.trim() == "name").then(|| value.trim().trim_matches('"').replace('-', "_"))
        })
}

/// A new project as `amble new app` writes it, for the linter's tests: the files the conventions
/// are checked on, with the counter example if `example`.
#[cfg(test)]
pub(crate) fn fixture(example: bool) -> std::collections::BTreeMap<String, String> {
    use super::register::{append, register};
    use super::{node, walk};

    let named = |template: &str| template.replace("{{package}}", "app");
    let registered = |source: &str, name: &str| register(source, name).unwrap();
    let extra = if example { "rand = \"0.9\"\n" } else { "" };
    let mut files = std::collections::BTreeMap::new();
    let mut put = |path: &str, contents: String| files.insert(path.to_string(), contents);
    put(
        "Cargo.toml",
        named(include_str!("../../../templates/rust/Cargo.toml")).replace("{{extra}}", extra),
    );
    put(
        "src/lib.rs",
        named(include_str!("../../../templates/rust/lib.rs")),
    );
    put("src/engine.rs", include_str!("../../engine.rs").to_string());
    let mut nodes = include_str!("../../../templates/rust/nodes_mod.rs").to_string();
    let mut walks = include_str!("../../../templates/rust/walks_mod.rs").to_string();
    let mut utils = include_str!("../../../templates/rust/utils_mod.rs").to_string();
    if example {
        for (name, source) in [
            (
                "count",
                include_str!("../../../templates/rust/example/count.rs"),
            ),
            (
                "start",
                include_str!("../../../templates/rust/example/start.rs"),
            ),
            (
                "stop",
                include_str!("../../../templates/rust/example/stop.rs"),
            ),
        ] {
            nodes = registered(&nodes, name);
            put(&format!("src/nodes/{name}.rs"), source.to_string());
        }
        nodes = append(&nodes, &node::has_trait("count", "i32"));
        utils = registered(&utils, "terminal");
        walks = registered(&walks, "counter");
        put(
            "src/utils/terminal.rs",
            include_str!("../../../templates/rust/example/terminal.rs").to_string(),
        );
        put(
            "src/walks/counter.rs",
            include_str!("../../../templates/rust/example/counter.rs").to_string(),
        );
        put(
            "specs/counter.md",
            include_str!("../../../templates/counter.md").to_string(),
        );
        put("src/main.rs", walk::runner("app", "counter"));
    }
    put("src/nodes/mod.rs", nodes);
    put("src/walks/mod.rs", walks);
    put("src/utils/mod.rs", utils);
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_crate_name_from_the_package_table() {
        let toml = "[workspace]\nname = \"no\"\n\n[package]\nname = \"my-app\"\n\n[dependencies]\n";
        assert_eq!(crate_name(toml).as_deref(), Some("my_app"));
        assert_eq!(crate_name("[dependencies]\nname = \"no\"\n"), None);
    }
}
