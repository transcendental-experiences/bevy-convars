//! Builtin CVars that are automatically registered in any application.

use std::ops::Deref;

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