//! The bevy-convars prelude with the most common types.
pub use crate::CVarFlags;
pub use crate::CVarsPlugin;

pub use crate::defaults::IsDefault;

#[cfg(feature = "config_loader")]
pub use crate::loader::CVarLoaderPluginBuilder;

#[cfg(feature = "config_loader")]
pub use crate::loader::CVarLoaderPlugin;

#[cfg(feature = "parse_cvars")]
pub use crate::parse::CVarOverride;

#[cfg(feature = "parse_cvars")]
pub use crate::save::CVarSaveContext;

pub use crate::WorldExtensions;
