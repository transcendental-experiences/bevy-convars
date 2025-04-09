//! Provides the ability to load TOML configuration files as a collection of CVars.
//!
//! # Recommendations
//! No default for the user's config file is provided, however one can use the [directories](https://crates.io/crates/directories) library to get platform-specific locations for those files.
//!

use std::{fs::File, io::Read, path::PathBuf};

use bevy_app::Plugin;
#[cfg(feature = "config_loader_asset")]
use bevy_asset::{AssetPath, AssetServer, WaitForAssetError};
use bevy_ecs::world::World;
use bevy_log::warn;
use serde::de::IntoDeserializer;
use toml_edit::{ImDocument, TomlError};

#[cfg(feature = "config_loader_asset")]
mod assets;
mod cvar_doc;
#[cfg(test)]
mod tests;

#[cfg(feature = "config_loader_asset")]
pub use assets::*;

pub use cvar_doc::*;

use crate::{CVarError, CVarManagement, WorldExtensions};

/// A config loader, which injests [DocumentContext]s and applies them to the world.
#[derive(Default)]
pub struct ConfigLoader {}

/// Methods for creating a config loader.
impl ConfigLoader {
    /// Applies a given config to the world.
    pub fn apply<S: AsRef<str>>(
        &self,
        world: &mut World,
        document: DocumentContext<S>,
    ) -> Result<(), CVarError> {
        let scanner = CVarDocScanner::new(document);

        let cvars: Vec<(&str, toml_edit::Item)> =
            scanner.find_cvars(world.resource::<CVarManagement>());

        for (cvar, value) in cvars {
            if let toml_edit::Item::Value(value) = value {
                world.set_cvar_deserialize(cvar, IntoDeserializer::into_deserializer(value))?;
            } else {
                warn!("CVar {cvar} couldn't be parsed, as it wasn't value-compatible.");
            }
        }

        Ok(())
    }

    /// Applies a given config to the world, by parsing it into a TOML document and [ConfigLoader::apply]ing that.
    pub fn apply_from_string(
        &self,
        world: &mut World,
        document: &str,
        source: Option<&str>,
    ) -> Result<(), CVarError> {
        let document = ImDocument::parse(document)?;

        let document = DocumentContext::new(document, source.unwrap_or("NO_SOURCE").to_owned());

        self.apply(world, document)?;

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

/// A builder to create a new [CVarLoaderPlugin]
#[derive(Default)]
pub struct CVarLoaderPluginBuilder {
    #[cfg(feature = "config_loader_asset")]
    layers_root: Option<AssetPath<'static>>,
    /// The user's config file within the OS filesystem
    #[cfg(feature = "config_loader_fs")]
    user_config_file: Option<PathBuf>,
    /// Any asset-managed config layers to load at startup.
    #[cfg(feature = "config_loader_asset")]
    asset_layers: Vec<PathBuf>,
    /// Any extra layers to load at startup.
    extra_layers: Vec<DocumentContext<String>>,
}

impl CVarLoaderPluginBuilder {
    /// The fancy default, loading layers from the asset path `ConfigLayers/` and automatically loading the default layers.
    /// Does not set the user config file path or add any extra layers.
    pub fn fancy() -> Self {
        Self {
            #[cfg(feature = "config_loader_asset")]
            layers_root: Some(AssetPath::parse("ConfigLayers/")),
            ..Default::default()
        }
        .load_default_layers()
    }

    /// Adds the default layers to the load list.
    /// The following layers are 'default' and may be added depending on build configuration:
    ///
    /// - `debug_assertions.toml` for `cfg(debug_assertions)`
    #[cfg(feature = "config_loader_asset")]
    pub fn load_default_layers(mut self) -> Self {
        #[cfg(debug_assertions)]
        self.asset_layers
            .push(PathBuf::from("debug_assertions.toml"));

        self
    }

    /// Conditionally adds an asset layer, meant to be used with [cfg!] or other conditions.
    /// You should prefer actual rust `if` statements for anything complex.
    ///
    /// ```
    /// # #![allow(unexpected_cfgs)]
    /// # use bevy_convars::*;
    /// # use bevy_convars::loader::*;
    ///
    /// let builder =
    ///     CVarLoaderPluginBuilder::fancy()
    ///         .add_asset_layer_if(cfg!(feature = "dev_tools"), "dev_tools.toml")
    ///         .add_asset_layer_if(cfg!(feature = "release"), "release.toml");
    ///
    /// ```
    #[cfg(feature = "config_loader_asset")]
    pub fn add_asset_layer_if(mut self, given: bool, layer: &'static str) -> Self {
        if given {
            self.asset_layers.push(PathBuf::from(layer));
        }

        self
    }

    /// Adds an asset layer to the builder.
    #[cfg(feature = "config_loader_asset")]
    pub fn add_asset_layer(mut self, layer: &'static str) -> Self {
        self.asset_layers.push(PathBuf::from(layer));

        self
    }

    /// Sets the root for config layers.
    #[cfg(feature = "config_loader_asset")]
    pub fn with_layers_root(self, path: AssetPath<'static>) -> Self {
        Self {
            layers_root: Some(path),
            ..self
        }
    }

    /// Sets the user config file location.
    #[cfg(feature = "config_loader_fs")]
    pub fn with_user_config_file(self, path: PathBuf) -> Self {
        Self {
            user_config_file: Some(path),
            ..self
        }
    }

    /// Adds a layer to load from the layer root. This should be a file relative to the root.
    #[cfg(feature = "config_loader_asset")]
    pub fn with_asset_layer(mut self, path: PathBuf) -> Self {
        self.asset_layers.push(path);

        self
    }

    /// Adds a pre-parsed config layer to apply.
    pub fn add_layer(mut self, layer: DocumentContext<String>) -> Self {
        self.extra_layers.push(layer);

        self
    }

    /// Consumes the builder to create a [CVarLoaderPlugin].
    pub fn build(self) -> CVarLoaderPlugin {
        if !self.asset_layers.is_empty() {
            assert!(
                self.layers_root.is_some(),
                "Can't add asset layers without a root."
            );
        }

        CVarLoaderPlugin {
            layers_root: self.layers_root,
            user_config_file: self.user_config_file,
            asset_layers: self.asset_layers,
            extra_layers: self.extra_layers,
        }
    }
}

/// Plugin that provides layered config loading for CVars, and additionally manages the user config file.
///
/// During build, the plugin will load any layers it was configured to load, and also any asset layers named by [ConfigLayers](crate::builtin::ConfigLayers)
///
/// # Remarks
/// This plugin **MUST** be added after all other CVar registering plugins. It's recommended to seperate CVar registration from other plugin registration to ensure it's done first.
pub struct CVarLoaderPlugin {
    /// The built-in layers root folder within assets.
    #[cfg(feature = "config_loader_asset")]
    layers_root: Option<AssetPath<'static>>,
    /// The user's config file within the OS filesystem.
    #[cfg(feature = "config_loader_fs")]
    user_config_file: Option<PathBuf>,
    /// Any asset-managed config layers to load at startup.
    #[cfg(feature = "config_loader_asset")]
    asset_layers: Vec<PathBuf>,
    /// Any extra layers to load at startup.
    extra_layers: Vec<DocumentContext<String>>,
}

impl Plugin for CVarLoaderPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        let loader = ConfigLoader::default();
        // Begin with any extra layers.

        for layer in self.extra_layers.iter() {
            let res = loader.apply(app.world_mut(), layer.clone());

            if let Err(e) = res {
                warn!(
                    "Failed to load an extra layer ({}), got error: {}",
                    layer.source(),
                    e
                );
            }
        }

        #[cfg(feature = "config_loader_asset")]
        {
            let server = app.world().resource::<AssetServer>().clone();
            for layer in self.asset_layers.iter() {
                let root = self.layers_root.as_ref().unwrap().clone();

                let path = root
                    .resolve(layer.to_str().unwrap())
                    .expect("Trying to resolve an asset layer should never fail.");

                let handle = server.load::<CVarConfig>(&path);

                match bevy_tasks::block_on(server.wait_for_asset(&handle)) {
                    Ok(()) => {}
                    Err(WaitForAssetError::Failed(err)) => {
                        match &*err {
                            bevy_asset::AssetLoadError::AssetReaderError(_) => {
                                bevy_log::warn!("Couldn't find config layer {layer:?}, skipping.")
                            }
                            e => bevy_log::error!(
                                "Failed to load the config layer {layer:?}, reason: {e}"
                            ),
                        }
                        continue;
                    }
                    Err(e) => {
                        bevy_log::error!("Failed to load the config layer {layer:?}, reason: {e}");
                        continue;
                    }
                }

                let res = loader.apply_asset(app.world_mut(), handle);

                if let Err(e) = res {
                    warn!(
                        "Failed to load an asset layer ({:?}), got error: {}",
                        path, e
                    );
                }
            }
        }

        #[cfg(feature = "config_loader_fs")]
        {
            if let Some(ref path) = self.user_config_file {
                let res = File::options().read(true).create(true).append(true).open(path);

                if let Err(e) = res {
                    warn!("Failed to create or open the user config file at {path:?}, got error: {e}");
                } else if let Ok(mut file) = res {
                    let mut buf = String::new();
                    file.read_to_string(&mut buf).unwrap();

                    let res = loader.apply_from_string(app.world_mut(), &buf, Some(&path.to_string_lossy()));

                    if let Err(e) = res {
                        warn!(
                            "Failed to load the user's config file ({:?}), got error: {}",
                            path, e
                        );
                    }
                }
            }
        }
    }
}
