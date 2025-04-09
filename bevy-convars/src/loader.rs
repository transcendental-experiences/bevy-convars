//! Provides the ability to load TOML configuration files as a collection of CVars.

use bevy_ecs::world::World;
use bevy_log::warn;
use serde::de::IntoDeserializer;
use toml_edit::TomlError;

mod cvar_doc;
#[cfg(test)]
mod tests;

pub use cvar_doc::*;

use crate::{CVarError, CVarManagement, WorldExtensions};

/// A config loader, which injests [DocumentContext]s and applies them to the world.
#[derive(Default)]
pub struct ConfigLoader {}

/// Methods for creating a config loader.
impl ConfigLoader {
    /// Applies a given config to the world.
    pub fn apply<S: AsRef<str>>(&self, world: &mut World, document: DocumentContext<S>) -> Result<(), CVarError> {
        let scanner = CVarDocScanner::new(document);

        let cvars: Vec<(&str, toml_edit::Item)> = scanner.find_cvars(world.resource::<CVarManagement>());

        for (cvar, value) in cvars {
            if let toml_edit::Item::Value(value) = value {
                world.set_cvar_deserialize(cvar, IntoDeserializer::into_deserializer(value))?;
            } else {
                warn!("CVar {cvar} couldn't be parsed, as it wasn't value-compatible.");
            }
        }

        Ok(())
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
