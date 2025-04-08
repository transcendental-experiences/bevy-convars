//! Builtin CVars that are automatically registered in any application.

use std::{ops::Deref, path::PathBuf};

use bevy_app::Plugin;
use bevy_ecs::prelude::Resource;
use bevy_ecs::system::SystemParam;

use crate::{CVarFlags, cvar_collection};
cvar_collection! {
    /// Collection of core CVars you can use as a system argument.
    pub struct CoreCVars & CoreCVarsMut {
        /// Enables logging ALL cvar modifications. This will log the change as info.
        log_cvar_changes = cvar LogCVarChanges("core.log_cvar_changes", CVarFlags::RUNTIME): bool = false,
    }

    /// Plugin that handles registering all the core CVars.
    #[doc(hidden)]
    pub struct CoreCVarsPlugin;
}

static_assertions::assert_impl_all!(CoreCVars: SystemParam);
static_assertions::assert_impl_all!(CoreCVarsPlugin: Plugin);
static_assertions::assert_impl_all!(LogCVarChanges: Resource, Deref<Target = bool>);

#[cfg(feature = "config_loader")]
cvar_collection! {
    /// Collection of config-loader related CVars you can use as a system parameter.
    pub struct ConfigLoaderCVars & ConfigLoaderCVarsMut {
        /// Names of configuration layer files to load in atop the default config.
        /// # Remarks
        /// Unlike basically all other CVars, this one cannot be set by file layers, because it defines them.
        config_layers = cvar ConfigLayers("core.config_layers", CVarFlags::LOCAL): Vec<PathBuf> = vec![],
    }

    /// Plugin that handles registering all the config loader CVars.
    #[doc(hidden)]
    pub struct ConfigLoaderCVarsPlugin;
}
