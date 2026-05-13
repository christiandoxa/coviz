use std::path::Path;
use std::str::FromStr;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

/// Source languages supported by the analyzer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Go,
    Rust,
}

impl Language {
    pub fn from_extension(path: impl AsRef<Path>) -> Option<Self> {
        match path.as_ref().extension().and_then(|ext| ext.to_str()) {
            Some("go") => Some(Self::Go),
            Some("rs") => Some(Self::Rust),
            _ => None,
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Go => "go",
            Self::Rust => "rs",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Go => "go",
            Self::Rust => "rust",
        }
    }
}

impl FromStr for Language {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "go" | "golang" => Ok(Self::Go),
            "rs" | "rust" => Ok(Self::Rust),
            _ => bail!("unsupported language: {value}"),
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::Language;

    #[test]
    fn detects_language_from_extension() {
        assert_eq!(Language::from_extension("main.go"), Some(Language::Go));
        assert_eq!(Language::from_extension("lib.rs"), Some(Language::Rust));
        assert_eq!(Language::from_extension("README.md"), None);
    }
}
