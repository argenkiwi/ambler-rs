//! Pure text generation and analysis: source in, source out, no I/O.
//!
//! Nodes decide what to do; the functions here work out what the files should say.

pub mod args;
pub mod lint;
pub mod names;
pub mod rust;
pub mod walk;

pub use lint::{Violation, lint};

/// One thing to do to the file system. Paths are relative to the current directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    /// Write `contents` to `path`, creating its folders. `format` says whether rustfmt runs on
    /// it afterwards.
    Write {
        path: String,
        contents: String,
        format: bool,
    },
}

impl Change {
    /// A file written as is.
    #[must_use]
    pub fn write(path: impl Into<String>, contents: impl Into<String>) -> Self {
        Self::Write {
            path: path.into(),
            contents: contents.into(),
            format: false,
        }
    }

    /// A generated Rust file, to be run through rustfmt once written.
    #[must_use]
    pub fn rust(path: impl Into<String>, contents: impl Into<String>) -> Self {
        Self::Write {
            path: path.into(),
            contents: contents.into(),
            format: true,
        }
    }

    /// The path this change touches.
    #[must_use]
    pub fn path(&self) -> &str {
        match self {
            Self::Write { path, .. } => path,
        }
    }
}

/// Why a verb cannot make its plan, and so which exit code it ends with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rejection {
    /// The arguments or the project do not allow it. Exit code 2.
    Invalid(String),
    /// The thing to create already exists. Exit code 1.
    Exists(String),
}

impl Rejection {
    /// The exit code of the Ambler Tooling Protocol for this rejection.
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Exists(_) => 1,
            Self::Invalid(_) => 2,
        }
    }

    /// Why the verb was rejected.
    #[must_use]
    pub fn into_message(self) -> String {
        match self {
            Self::Invalid(message) | Self::Exists(message) => message,
        }
    }
}

impl From<String> for Rejection {
    fn from(message: String) -> Self {
        Self::Invalid(message)
    }
}

impl From<&str> for Rejection {
    fn from(message: &str) -> Self {
        Self::Invalid(message.to_string())
    }
}

/// What a verb is about to do, and what to tell the caller once it is done.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Plan {
    pub changes: Vec<Change>,
    /// Next steps, printed after the changes are applied.
    pub hint: String,
}
