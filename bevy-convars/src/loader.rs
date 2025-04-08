//! Provides a layered config loader that can load multiple configs over top one another.

use std::{path::Path, str::FromStr};

use toml_edit::{DocumentMut, TomlError};

mod cvar_doc;

/// The layered config loader plugin.
/// # Remarks
/// This should be added AFTER all CVar plugins have been registered.
pub struct ConfigLoaderPlugin {
    sources: Vec<DocumentMut>,
}

/// Methods for creating a config loader.
impl ConfigLoaderPlugin {
    /// Create a config loader from an ordered list of TOML-containing strings.
    pub fn from_strs(
        sources: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<Self, ConfigLoaderError> {
        let documents: Result<Vec<_>, _> = sources
            .into_iter()
            .map(|s| DocumentMut::from_str(s.as_ref()))
            .collect();

        Ok(Self {
            sources: documents?,
        })
    }

    /// Create a config loader from an ordered list of toml files to load.
    #[cfg(feature = "config_loader_fs")]
    pub fn from_files(
        sources: impl IntoIterator<Item = impl AsRef<Path>>,
    ) -> Result<Self, ConfigLoaderError> {
        let mut source_contents = Vec::new();

        for path in sources.into_iter() {
            let data = std::fs::read_to_string(path)?;
            source_contents.push(data);
        }

        let documents: Result<Vec<_>, _> = source_contents
            .into_iter()
            .map(|s| DocumentMut::from_str(&s))
            .collect();

        Ok(Self {
            sources: documents?,
        })
    }
}

/// A non-recoverable error that can occur when loading configuration.
#[derive(thiserror::Error, Debug)]
pub enum ConfigLoaderError {
    /// Wrapper over an inner parsing error.
    #[error(transparent)]
    ParseError(TomlError),
    /// Wrapper over an inner IO error.
    #[error(transparent)]
    IoError(std::io::Error),
}

impl From<TomlError> for ConfigLoaderError {
    fn from(value: TomlError) -> Self {
        Self::ParseError(value)
    }
}

impl From<std::io::Error> for ConfigLoaderError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value)
    }
}
