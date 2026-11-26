//! The guides `cargo ambler help [topic]` prints, compiled into the binary.
//!
//! They are the only place the conventions of a Rust Ambler project are written down, and they
//! ship with the CLI that enforces them, so a project reads the guides of the version it pins.

/// Every topic `cargo ambler help <topic>` knows: the verbs, then the conventions behind them.
pub const TOPICS: &[&str] = &[
    "node",
    "util",
    "walk",
    "check",
    "conventions",
    "node-design",
    "testing",
    "spec",
];

/// The guide for `topic`, or the general guide for `""`.
#[must_use]
pub fn guide(topic: &str) -> Option<&'static str> {
    Some(match topic {
        "" => include_str!("guides/general.md"),
        "node" => include_str!("guides/node.md"),
        "util" => include_str!("guides/util.md"),
        "walk" => include_str!("guides/walk.md"),
        "check" => include_str!("guides/check.md"),
        "conventions" => include_str!("guides/conventions.md"),
        "node-design" => include_str!("guides/node-design.md"),
        "testing" => include_str!("guides/testing.md"),
        "spec" => include_str!("guides/spec.md"),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_topic_has_a_guide() {
        for topic in TOPICS {
            assert!(guide(topic).is_some(), "{topic}");
        }
    }

    #[test]
    fn no_guide_tells_the_reader_to_run_a_global_ambler_verb() {
        for topic in TOPICS.iter().chain(&[""]) {
            let text = guide(topic).unwrap_or_default();
            for verb in ["node", "util", "walk", "check", "help"] {
                let global = format!("`ambler {verb}");
                assert!(!text.contains(&global), "{topic}: {global}");
            }
        }
    }
}
