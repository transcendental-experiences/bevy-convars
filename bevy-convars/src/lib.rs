//! Provides an implementation of ConVars, a form of global configuration for an application.
//!
//! Intended for full applications, not for libraries!
//! If you're a library author, the easiest and best way to integrate is simply to make your library configurable, and allow the end user to create convars themselves.
//!
//!
//! # Example
//! ```ignore
//! crate::cvar_collection! {
//!     pub struct RenderCVars & RenderCVarsMut {
//!         enable_xr = cvar EnableXr("render.enable_xr", CVarFlags::SAVED): bool = false,
//!         enable_renderdoc = cvar EnableRenderdoc("render.enable_renderdoc", CVarFlags::LOCAL): bool = false,
//!
//!         /*
//!         *   Anti-aliasing
//!         */
//!         aa_method = cvar AaMethod("render.aa.method", CVarFlags::SAVED | CVarFlags::RUNTIME): AntialiasMethod = AntialiasMethod::Fxaa,
//!
//!         fxaa_sensitivity = cvar FxaaSensitivty("render.aa.fxaa_sensitivity", CVarFlags::SAVED | CVarFlags::RUNTIME): FxaaSensitivity = FxaaSensitivity::Medium,
//!
//!         msaa_samples = cvar MsaaSamples("render.aa.msaa_samples", CVarFlags::SAVED | CVarFlags::RUNTIME): MsaaSamplingConfig = MsaaSamplingConfig::Msaa4,
//!
//!         /*
//!         *   SSAO.
//!         */
//!         enable_ssao = cvar EnableSsao("render.ssao.enabled", CVarFlags::SAVED | CVarFlags::RUNTIME): bool = true,
//!         ssao_quality = cvar RenderSsaoQuality("render.ssao.quality", CVarFlags::SAVED | CVarFlags::RUNTIME): SsaoQuality = SsaoQuality::High,
//!         ssao_object_thickness = cvar SsaoObjectThickness("render.ssao.object_thickness", CVarFlags::SAVED | CVarFlags::RUNTIME): f32 = 0.25
//!     }
//!
//!     pub struct RenderCVarsPlugin;
//!}
//!
//!  ...
//!
//! fn sync_cvars_to_camera(
//!    cameras: Query<(Entity, Ref<SettingsAwareCamera>)>,
//!    cvars: RenderCVars,
//!    enable_ssao: Res<EnableSsao>,
//!    mut commands: Commands,
//!) {
//!    // Can read directly out of the RenderCVars param..
//!    let aa_method = **cvars.aa_method;
//!
//!    // or from a specific cvar resource.
//!    // All CVar types implement Deref and DerefMut for their inner type to make them easy to unpack and modify.
//!    let ssao_on = **enable_ssao;
//!
//!    ...
//!}
//!```

#![deny(missing_docs)]

use bevy_app::App;
use bevy_app::prelude::*;
use bevy_ecs::component::ComponentId;
use bevy_ecs::prelude::*;
use bevy_platform_support::collections::HashMap;
use bevy_reflect::{TypeRegistration, prelude::*};
#[cfg(feature = "config_loader")]
use builtin::ConfigLoaderCVarsPlugin;
use builtin::CoreCVarsPlugin;
use builtin::LogCVarChanges;
#[cfg(feature = "parse_cvars")]
use parse::CVarOverride;
use reflect::CVarMeta;
use serde::Deserializer;
#[cfg(feature = "parse_cvars")]
use serde::de::IntoDeserializer as _;

pub mod defaults;
mod error;
mod macros;
mod types;
pub use error::*;
pub use types::*;
pub mod builtin;
#[cfg(feature = "config_loader")]
pub mod loader;
#[cfg(feature = "parse_cvars")]
pub mod parse;
pub mod reflect;

#[cfg(test)]
mod tests;

/// Internal re-exports to avoid depending on the user's scope.
#[doc(hidden)]
pub mod reexports {
    pub use bevy_app;
    pub use bevy_ecs;
    pub use bevy_reflect;
    pub mod jank {
        pub use crate::reflect::ReflectCVar as ReflectCVar__MACRO_JANK;
        pub use bevy_ecs::reflect::ReflectResource as ReflectResource__MACRO_JANK;
        pub use bevy_reflect::prelude::ReflectDefault as ReflectDefault__MACRO_JANK;
    }
}

/// Core plugin for providing CVars.
/// # Remarks
/// Needs to be registered before any of the generated plugins to ensure [CVarManagement] is available.
pub struct CVarsPlugin;

#[derive(Debug)]
pub(crate) enum CVarTreeNode {
    Leaf {
        name: &'static str,
        reg: ComponentId,
    },
    Branch {
        descendants: HashMap<&'static str, CVarTreeNode>,
    },
}

impl Default for CVarTreeNode {
    fn default() -> Self {
        CVarTreeNode::Branch {
            descendants: Default::default(),
        }
    }
}

struct CVarTreeEditContext {
    new_cvar: &'static str,
}

impl CVarTreeNode {
    pub fn children(&self) -> Option<impl Iterator<Item = (&'_ &'static str, &'_ CVarTreeNode)>> {
        match self {
            CVarTreeNode::Leaf { name: _, reg: _ } => None,
            CVarTreeNode::Branch { descendants } => Some(descendants.iter()),
        }
    }

    pub fn is_leaf(&self) -> bool {
        matches!(self, CVarTreeNode::Leaf { .. })
    }

    pub fn insert(&mut self, name: &'static str, id: ComponentId) {
        let segments: Vec<&'static str> = name.split('.').collect();
        let edit_ctx = CVarTreeEditContext { new_cvar: name };

        let mut cur = self;
        for (idx, segment) in segments.iter().enumerate() {
            if idx == segments.len() - 1 {
                let _ = cur.insert_leaf(segment, id, &edit_ctx);
                return;
            } else {
                cur = cur.get_or_insert_branch(segment, &edit_ctx);
            }
        }
    }

    #[must_use]
    fn get_or_insert_branch(
        &mut self,
        key: &'static str,
        ctx: &CVarTreeEditContext,
    ) -> &mut CVarTreeNode {
        match self {
            CVarTreeNode::Leaf { name, reg: _ } => panic!(
                "Tried to insert branch {name} into a terminating node. A CVar cannot be both a value and table. CVar in question is {}",
                ctx.new_cvar
            ),
            CVarTreeNode::Branch { descendants } => {
                descendants.entry(key).or_insert(CVarTreeNode::Branch {
                    descendants: Default::default(),
                })
            }
        }
    }

    #[must_use]
    fn insert_leaf(
        &mut self,
        key: &'static str,
        reg: ComponentId,
        ctx: &CVarTreeEditContext,
    ) -> &mut CVarTreeNode {
        match self {
            CVarTreeNode::Leaf { name, reg: _ } => {
                panic!(
                    "Tried to insert leaf {name} into a terminating node. Is there a duplicate or overlap? CVar in question is {}",
                    ctx.new_cvar
                )
            }
            CVarTreeNode::Branch { descendants } => {
                assert!(
                    descendants
                        .insert(
                            key,
                            CVarTreeNode::Leaf {
                                name: ctx.new_cvar,
                                reg
                            }
                        )
                        .is_none(),
                    "Attempted to insert a duplicate CVar. CVar in question is {}",
                    ctx.new_cvar
                );

                descendants.get_mut(key).unwrap()
            }
        }
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<ComponentId> {
        let mut cur = self;
        for seg in name.split('.') {
            let CVarTreeNode::Branch { descendants } = cur else {
                return None;
            };

            cur = descendants.get(seg)?;
        }

        let CVarTreeNode::Leaf { name: _, reg } = cur else {
            return None;
        };

        Some(*reg)
    }
}

/// App resource that provides management information and functionality for CVars.
#[derive(Default, Resource)]
pub struct CVarManagement {
    /// An index of all cvar resources and their type registrations.
    pub(crate) resources: HashMap<ComponentId, TypeRegistration>,
    /// An index of all CVars and their types.
    pub(crate) tree: CVarTreeNode,
}

impl CVarManagement {
    /// Register a CVar of the given type to the internal storage.
    #[doc(hidden)]
    pub fn register_cvar<T: Reflect + Resource + CVarMeta>(&mut self, app: &mut App) {
        let registration = {
            let registry = app.world().resource::<AppTypeRegistry>();
            let registry = registry.read();
            registry.get(::std::any::TypeId::of::<T>()).unwrap().clone()
        };
        let cid = app.world().resource_id::<T>().unwrap();

        self.tree.insert(T::CVAR_PATH, cid);
        self.resources.insert(cid, registration);
    }

    /// Gets a CVar's value through reflection.
    /// # Remarks
    /// This returns the inner value, not the cvar resource itself.
    pub fn get_cvar_reflect<'a>(
        &self,
        world: &'a World,
        cvar: &str,
    ) -> Result<&'a dyn Reflect, CVarError> {
        let cid = self.tree.get(cvar).ok_or(CVarError::UnknownCVar)?;

        let ty_info = self.resources.get(&cid).ok_or(CVarError::UnknownCVar)?;

        let reflect_res = ty_info
            .data::<ReflectResource>()
            .ok_or(CVarError::BadCVarType)?;
        let reflect_cvar = ty_info
            .data::<reflect::ReflectCVar>()
            .ok_or(CVarError::BadCVarType)?;

        let res = reflect_res.reflect(world)?;

        reflect_cvar
            .reflect_inner(res.as_partial_reflect())
            .unwrap()
            .try_as_reflect()
            .ok_or(CVarError::BadCVarType)
    }

    /// Gets a CVar's value mutably through reflection.
    /// # Remarks
    /// This returns the inner value, not the cvar resource itself.
    /// A change-detection aware handle is returned.
    pub fn get_cvar_reflect_mut<'a>(
        &self,
        world: &'a mut World,
        cvar: &str,
    ) -> Result<Mut<'a, dyn Reflect>, CVarError> {
        let cid = self.tree.get(cvar).ok_or(CVarError::UnknownCVar)?;

        let ty_info = self.resources.get(&cid).ok_or(CVarError::UnknownCVar)?;

        let reflect_res = ty_info
            .data::<ReflectResource>()
            .ok_or(CVarError::BadCVarType)?;
        let reflect_cvar = ty_info
            .data::<reflect::ReflectCVar>()
            .ok_or(CVarError::BadCVarType)?;

        Ok(reflect_res.reflect_mut(world)?.map_unchanged(|x| {
            reflect_cvar
                .reflect_inner_mut(x.as_partial_reflect_mut())
                .unwrap()
                .try_as_reflect_mut()
                .unwrap()
        }))
    }

    /// Set a CVar to the given reflected value using reflection.
    /// # Remarks
    /// Use the WorldExtensions version if you can, it handles the invariants. This is harder to call than it looks due to needing mutable world.
    pub fn set_cvar_reflect(
        &self,
        world: &mut World,
        cvar: &str,
        value: &dyn Reflect,
    ) -> Result<(), CVarError> {
        let cid = self.tree.get(cvar).ok_or(CVarError::UnknownCVar)?;

        let ty_reg = self.resources.get(&cid).ok_or(CVarError::MissingCid)?;

        let reflect_cvar = ty_reg.data::<reflect::ReflectCVar>().unwrap();

        let reflect_res = ty_reg.data::<ReflectResource>().unwrap();

        let cvar = reflect_res.reflect_mut(world)?;

        reflect_cvar.reflect_apply(
            cvar.into_inner().as_partial_reflect_mut(),
            value.as_partial_reflect(),
        )?;

        Ok(())
    }

    /// Set a CVar to the given reflected value using reflection, without triggering change detection.
    /// # Remarks
    /// Use the WorldExtensions version if you can, it handles the invariants. This is harder to call than it looks due to needing mutable world.
    pub fn set_cvar_reflect_no_change(
        &self,
        world: &mut World,
        cvar: &str,
        value: &dyn Reflect,
    ) -> Result<(), CVarError> {
        let cid = self.tree.get(cvar).ok_or(CVarError::UnknownCVar)?;

        let ty_reg = self.resources.get(&cid).ok_or(CVarError::MissingCid)?;

        let reflect_cvar = ty_reg.data::<reflect::ReflectCVar>().unwrap();

        let reflect_res = ty_reg.data::<ReflectResource>().unwrap();

        let mut cvar = reflect_res.reflect_mut(world)?;

        reflect_cvar.reflect_apply(
            cvar.bypass_change_detection().as_partial_reflect_mut(),
            value.as_partial_reflect(),
        )?;

        Ok(())
    }

    /// Set a CVar to the given deserializable value using reflection.
    /// # Remarks
    /// Use the WorldExtensions version if you can, it handles the invariants. This is harder to call than it looks due to needing mutable world.
    pub fn set_cvar_deserialize<'w, 'a>(
        &self,
        world: &mut World,
        cvar: &str,
        value: impl Deserializer<'a>,
    ) -> Result<(), CVarError> {
        let cid = self.tree.get(cvar).ok_or(CVarError::UnknownCVar)?;

        let ty_reg = self.resources.get(&cid).ok_or(CVarError::MissingCid)?;

        let reflect_cvar = ty_reg.data::<reflect::ReflectCVar>().unwrap();

        let value_patch = {
            let field_0 = reflect_cvar.inner_type();

            let registry = world.resource::<AppTypeRegistry>().read();

            let deserializer = registry
                .get(field_0)
                .ok_or(CVarError::BadCVarType)?
                .data::<ReflectDeserialize>()
                .ok_or(CVarError::CannotDeserialize)?;

            deserializer
                .deserialize(value)
                .map_err(|e| CVarError::FailedDeserialize(format!("{e:?}")))?
        };

        let reflect_res = ty_reg.data::<ReflectResource>().unwrap();

        let cvar = reflect_res.reflect_mut(world)?;

        reflect_cvar.reflect_apply(
            cvar.into_inner().as_partial_reflect_mut(),
            value_patch.as_partial_reflect(),
        )?;

        Ok(())
    }

    /// Set a CVar to the given deserializable value using reflection, without triggering change detection.
    /// # Remarks
    /// Use the WorldExtensions version if you can, it handles the invariants. This is harder to call than it looks due to needing mutable world.
    pub fn set_cvar_deserialize_no_change<'w, 'a>(
        &self,
        world: &mut World,
        cvar: &str,
        value: impl Deserializer<'a>,
    ) -> Result<(), CVarError> {
        let cid = self.tree.get(cvar).ok_or(CVarError::UnknownCVar)?;

        let ty_reg = self.resources.get(&cid).ok_or(CVarError::MissingCid)?;

        let reflect_cvar = ty_reg.data::<reflect::ReflectCVar>().unwrap();

        let value_patch = {
            let field_0 = reflect_cvar.inner_type();

            let registry = world.resource::<AppTypeRegistry>().read();

            let deserializer = registry
                .get(field_0)
                .ok_or(CVarError::CannotDeserialize)?
                .data::<ReflectDeserialize>()
                .ok_or(CVarError::CannotDeserialize)?;

            deserializer
                .deserialize(value)
                .map_err(|e| CVarError::FailedDeserialize(format!("{e:?}")))?
        };

        let reflect_res = ty_reg.data::<ReflectResource>().unwrap();

        let mut cvar = reflect_res.reflect_mut(world)?;

        reflect_cvar.reflect_apply(
            cvar.bypass_change_detection().as_partial_reflect_mut(),
            value_patch.as_partial_reflect(),
        )?;

        Ok(())
    }
}

/// Provides extensions to the world for CVars.
pub trait WorldExtensions {
    #[doc(hidden)]
    fn as_world(&mut self) -> &mut World;

    /// Set a CVar on the world through reflection, by deserializing the provided data into it.
    fn set_cvar_deserialize<'a>(
        &mut self,
        cvar: &str,
        value: impl serde::Deserializer<'a>,
    ) -> Result<(), CVarError> {
        let cell = self.as_world();

        cell.resource_scope::<CVarManagement, _>(|w, management| {
            management.set_cvar_deserialize(w, cvar, value)
        })
    }

    /// Set a CVar on the world through reflection by deserializing the provided data into it, without triggering change detection.
    fn set_cvar_deserialize_no_change<'a>(
        &mut self,
        cvar: &str,
        value: impl serde::Deserializer<'a>,
    ) -> Result<(), CVarError> {
        let cell = self.as_world();

        cell.resource_scope::<CVarManagement, _>(|w, management| {
            management.set_cvar_deserialize_no_change(w, cvar, value)
        })
    }

    /// Set a CVar on the world through reflection
    fn set_cvar_reflect(&mut self, cvar: &str, value: &dyn Reflect) -> Result<(), CVarError> {
        let cell = self.as_world();

        cell.resource_scope::<CVarManagement, _>(|w, management| {
            management.set_cvar_reflect(w, cvar, value)
        })
    }

    /// Set a CVar on the world through reflection, without triggering change detection.
    fn set_cvar_reflect_no_change(
        &mut self,
        cvar: &str,
        value: &dyn Reflect,
    ) -> Result<(), CVarError> {
        let cell = self.as_world();

        cell.resource_scope::<CVarManagement, _>(|w, management| {
            management.set_cvar_reflect_no_change(w, cvar, value)
        })
    }

    /// Set a CVar on the world using the provided override.
    /// # Remarks
    /// CVar overrides, by design, bypass change detection to look like the default value of the CVar.
    #[cfg(feature = "parse_cvars")]
    fn set_cvar_with_override(&mut self, r#override: &CVarOverride) -> Result<(), CVarError> {
        let cell = self.as_world();

        cell.resource_scope::<CVarManagement, _>(|w, management| {
            management.set_cvar_deserialize_no_change(
                w,
                &r#override.0,
                r#override.1.clone().into_deserializer(),
            )
        })
    }
}

impl WorldExtensions for World {
    fn as_world(&mut self) -> &mut World {
        self
    }
}

impl Plugin for CVarsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.register_type::<CVarFlags>();

        app.insert_resource::<CVarManagement>(CVarManagement::default());
        app.add_plugins(CoreCVarsPlugin);
        #[cfg(feature = "config_loader")]
        {
            app.add_plugins(ConfigLoaderCVarsPlugin);
        }
    }
}

/// Internal function meant for the macros. Don't use this!
/// Handles reporting CVar changes if LogCVarChanges is set.
#[doc(hidden)]
pub fn cvar_modified_system<T: CVarMeta>(
    r: bevy_ecs::prelude::Res<T>,
    log_updates: Res<LogCVarChanges>,
) {
    use bevy_ecs::prelude::DetectChanges as _;

    if **log_updates && r.is_changed() {
        bevy_log::info!("CVar modified: {} = {:?}", T::CVAR_PATH, **r);
    }

    if !r.is_changed() {
        return;
    }

    if !T::flags().contains(CVarFlags::RUNTIME) && !r.is_added() {
        if T::flags().contains(CVarFlags::SAVED) {
            bevy_log::warn!("Non-runtime CVar was modified! Change will not apply until restart.");
        } else {
            bevy_log::error!("Non-runtime, non-saved CVar was modified! This will have NO EFFECT.");
        }
    }
}
