use bevy_asset::{Asset, AssetLoader, Assets, AsyncReadExt as _, Handle};
use bevy_ecs::world::World;
use bevy_reflect::Reflect;
use std::error::Error;
use toml_edit::ImDocument;

use crate::CVarError;

use super::{ConfigLoader, DocumentContext};

impl ConfigLoader {
    /// Applies a given config to the world.
    pub fn apply_asset(
        &self,
        world: &mut World,
        asset: Handle<CVarConfig>,
    ) -> Result<(), CVarError> {
        let document = world
            .resource::<Assets<CVarConfig>>()
            .get(&asset)
            .unwrap()
            .clone()
            .0;

        self.apply(world, document, false)?;

        Ok(())
    }
}

/// A config as an asset, allowing you to use standard bevy asset loading.
#[derive(Asset, Reflect, Clone)]
pub struct CVarConfig(#[reflect(ignore)] pub(super) DocumentContext<String>);

/// Provides an asset loader for [CVarConfig].
#[derive(Default)]
pub struct ConfigAssetLoader {}

impl AssetLoader for ConfigAssetLoader {
    type Asset = CVarConfig;

    type Settings = ();

    type Error = Box<dyn Error + Send + Sync + 'static>;

    async fn load(
        &self,
        reader: &mut dyn bevy_asset::io::Reader,
        _: &Self::Settings,
        load_context: &mut bevy_asset::LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut buf = String::new();
        reader.read_to_string(&mut buf).await?;
        Ok(CVarConfig(DocumentContext::new(
            ImDocument::parse(buf)?,
            load_context.path().to_str().unwrap().to_owned(),
        )))
    }

    fn extensions(&self) -> &[&str] {
        &["toml"]
    }
}
