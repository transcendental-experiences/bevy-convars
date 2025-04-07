//! Provides an implementation of ConVars, a form of global configuration for an application.
//!
//! Intended for full applications, not for libraries!
//! If you're a library author, the easiest and best way to integrate is simply to make your library configurable, and allow the end user to create convars themselves.
//!
//!
//! # Example
//! ```rust
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

use std::path::PathBuf;

use bevy_app::App;
use bevy_app::prelude::*;
use bevy_ecs::component::ComponentId;
use bevy_ecs::prelude::*;
use bevy_reflect::{DynamicTupleStruct, TypeRegistration, prelude::*};
use bevy_utils::HashMap;
#[cfg(feature = "parse_cvars")]
use parse::CVarOverride;
use serde::de::IntoDeserializer as _;
use thiserror::Error;

mod types;
pub use types::*;
pub mod parse;

/// Internal re-exports to avoid depending on the user's scope.
#[doc(hidden)]
pub mod reexports {
    pub use bevy_app;
    pub use bevy_ecs;
    pub use bevy_reflect;
    pub mod jank {
        pub use bevy_ecs::reflect::ReflectResource as ReflectResource__CALL_CVARDECLIMPORTS;
        pub use bevy_reflect::prelude::ReflectDefault as ReflectDefault__CALL_CVARDECLIMPORTS;
    }
}

/// Core plugin for providing CVars.
/// # Remarks
/// Needs to be registered before any of the generated plugins to ensure [CVarManagement] is available.
pub struct CVarsPlugin;

#[derive(Debug)]
enum CVarTreeNode {
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

impl CVarTreeNode {
    pub fn insert(&mut self, name: &'static str, id: ComponentId) {
        let segments: Vec<&'static str> = name.split('.').collect();

        let mut cur = self;
        for (idx, segment) in segments.iter().enumerate() {
            if idx == segments.len() - 1 {
                cur.insert_leaf(segment, id);
            } else {
                cur = cur.get_or_insert_branch(segment);
            }
        }
    }

    fn get_or_insert_branch(&mut self, key: &'static str) -> &mut CVarTreeNode {
        match self {
            CVarTreeNode::Leaf { name, reg: _ } => panic!(
                "Tried to insert branch {name} into a terminating node. A CVar cannot be both a value and table."
            ),
            CVarTreeNode::Branch { descendants } => {
                descendants.entry(key).or_insert(CVarTreeNode::Branch {
                    descendants: Default::default(),
                })
            }
        }
    }

    fn insert_leaf(&mut self, key: &'static str, reg: ComponentId) -> &mut CVarTreeNode {
        match self {
            CVarTreeNode::Leaf { name, reg: _ } => panic!(
                "Tried to insert leaf {name} into a terminating node. A CVar cannot be both a value and table."
            ),
            CVarTreeNode::Branch { descendants } => descendants
                .entry(key)
                .or_insert(CVarTreeNode::Leaf { name: key, reg }),
        }
    }

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
    resources: HashMap<ComponentId, TypeRegistration>,
    /// An index of all CVars and their types.
    tree: CVarTreeNode,
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
    pub fn get_cvar_reflect<'a>(&self, world: &'a World, cvar: &str) -> Option<&'a dyn Reflect> {
        let cid = self.tree.get(cvar)?;

        let ty_info = self.resources.get(&cid)?;

        let reflect_res = ty_info.data::<ReflectResource>()?;

        let res = reflect_res.reflect(world)?;

        let val = res.reflect_ref().as_tuple_struct().unwrap().field(0)?;

        val.try_as_reflect()
    }

    /// Gets a CVar's value mutably through reflection.
    /// # Remarks
    /// This returns the inner value, not the cvar resource itself.
    pub fn get_cvar_reflect_mut<'a>(
        &self,
        world: &'a mut World,
        cvar: &str,
    ) -> Option<Mut<'a, dyn Reflect>> {
        let cid = self.tree.get(cvar)?;

        let ty_info = self.resources.get(&cid)?;

        let reflect_res = ty_info.data::<ReflectResource>()?;

        Some(reflect_res.reflect_mut(world)?.map_unchanged(|x| {
            x.reflect_mut()
                .as_tuple_struct()
                .unwrap()
                .field_mut(0)
                .unwrap()
                .try_as_reflect_mut()
                .unwrap()
        }))
    }

    /// Set a CVar to the given deserializable value using reflection.
    /// # Remarks
    /// Use the WorldExtensions version if you can, it handles the invariants. This is harder to call than it looks due to needing mutable world.
    pub fn set_cvar_reflect<'w, 'a>(
        &self,
        world: &mut World,
        cvar: &str,
        value: impl serde::Deserializer<'a>,
    ) -> Result<(), CVarError> {
        let cid = self.tree.get(cvar).ok_or(CVarError::UnknownCVar)?;

        let ty_reg = self.resources.get(&cid).ok_or(CVarError::MissingCid)?;

        let value_patch = {
            let tuple = ty_reg
                .type_info()
                .as_tuple_struct()
                .or(Err(CVarError::BadCVarType))?;

            let field_0 = tuple.field_at(0).ok_or(CVarError::BadCVarType)?;
            let field_0 = field_0.type_id();

            let registry = world.resource::<AppTypeRegistry>().read();

            let deserializer = registry
                .get(field_0)
                .ok_or(CVarError::CannotDeserialize)?
                .data::<ReflectDeserialize>()
                .ok_or(CVarError::CannotDeserialize)?;

            let field_0_val = deserializer
                .deserialize(value)
                .map_err(|e| CVarError::FailedDeserialize(format!("{e:?}")))?;

            let mut patch = DynamicTupleStruct::default();

            patch.insert_boxed(field_0_val);

            patch
        };

        let reflect_ptr = ty_reg.data::<ReflectResource>().unwrap();

        reflect_ptr.apply(world, &value_patch);

        Ok(())
    }
}

/// Errors that can occur when manipulating CVars.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum CVarError {
    /// Error indicating a CVar was never registered or is invalid.
    #[error("Unknown CVar.")]
    UnknownCVar,
    /// Error indicating the given CVar type is invalid.
    #[error(
        "CVar is not internally a Tuple Struct of the expected layout, did you try to register it manually?"
    )]
    BadCVarType,
    /// Error indicating the CVar type is missing a [ComponentId] and is likely not registered correctly.
    #[error("Missing ComponentID, was the resource registered?")]
    MissingCid,
    /// Error indicating the underlying type of the CVar cannot be deserialized, and as such cannot be reflected over.
    #[error("Underlying CVar type cannot be deserialized.")]
    CannotDeserialize,
    /// Error indicating the CVar failed to deserialize.
    #[error("Failed to deserialize.")]
    FailedDeserialize(String),
}

/// Provides extensions to the world for CVars.
pub trait WorldExtensions {
    #[doc(hidden)]
    fn as_world(&mut self) -> &mut World;

    /// Set a CVar on the world through reflection, by deserializing the provided data into it.
    fn set_cvar_reflect<'a>(
        &mut self,
        cvar: &str,
        value: impl serde::Deserializer<'a>,
    ) -> Result<(), CVarError> {
        let cell = self.as_world();

        cell.resource_scope::<CVarManagement, _>(|w, management| {
            management.set_cvar_reflect(w, cvar, value)
        })
    }

    /// Set a CVar on the world using the provided override.
    #[cfg(feature = "parse_cvars")]
    fn set_cvar_with_override(&mut self, r#override: &CVarOverride) -> Result<(), CVarError> {
        let cell = self.as_world();

        cell.resource_scope::<CVarManagement, _>(|w, management| {
            management.set_cvar_reflect(w, &r#override.0, r#override.1.clone().into_deserializer())
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

/// Declares an individual CVar. you probably want the collection macro instead.
#[macro_export]
#[doc(hidden)]
macro_rules! cvar {
    ($(#[$cvar_doc:meta])*
        $cvar_ident:ident($cvar_path:literal, $cvar_flags:expr): $cvar_ty:ty = $cvar_default:expr
    ) => {
        #[allow(unused_imports, reason = "Working around limitations of rust and bevy's macros.")]
        use $crate::reexports::jank::*;

        #[derive(
            $crate::reexports::bevy_reflect::Reflect,
        )]
        #[reflect(Default__CALL_CVARDECLIMPORTS, Resource__CALL_CVARDECLIMPORTS)]
        $(
            #[$cvar_doc]
        )*
        pub struct $cvar_ident($cvar_ty);

        impl  $crate::reexports::bevy_ecs::system::Resource for $cvar_ident { }

        impl ::std::ops::Deref for $cvar_ident {
            type Target = $cvar_ty;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl ::std::ops::DerefMut for $cvar_ident {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl ::std::default::Default for $cvar_ident {
            fn default() -> Self {
                Self($cvar_default)
            }
        }

        impl $crate::CVarMeta for $cvar_ident {
            type Inner = $cvar_ty;
            const CVAR_PATH: &'static str = $cvar_path;
            fn flags() -> CVarFlags {
                $cvar_flags
            }
        }
    };
}

/// Declares a collection of CVars.
/// # Example
/// ```rust
/// cvar_decl_imports!();
/// cvar_collection! {
///     /// A collection of exmaple CVars to use as a system param.
///     pub struct ExampleCVars & ExampleCVarsMut {
///         /// An example CVar declaration.
///         example_1 = cvar Example1("example.example_1", CVarFlags::SAVED): bool = true
///     }
///
///     /// The plugin to register the example CVars.
///     pub struct ExampleCVarsPlugin;
/// }
///
/// ```
#[macro_export]
macro_rules! cvar_collection {
    // macro moment
    {
        $(#[$collection_doc:meta])*
        $collection_vis:vis struct $cvar_collection_ident:ident & $cvar_collection_ident_mut:ident {
            $($(#[$cvar_doc:meta])*
                $field_name:ident = cvar $cvar_ident:ident($cvar_path:literal, $cvar_flags:expr): $cvar_ty:ty = $cvar_default:expr
            ),* $(,)?
        }

        $(#[$plugin_doc:meta])*
        $plugin_vis:vis struct $cvar_collection_plugin:ident;
    } => {
        $(#[$collection_doc])*
        #[derive($crate::reexports::bevy_ecs::system::SystemParam)]
        $collection_vis struct $cvar_collection_ident<'w> {
            $(
                #[allow(missing_docs)]
                pub $field_name: $crate::reexports::bevy_ecs::change_detection::Res<'w, $cvar_ident>
            ),*
        }

        $(#[$collection_doc])*
        #[derive($crate::reexports::bevy_ecs::system::SystemParam)]
        $collection_vis struct $cvar_collection_ident_mut<'w> {
            $(
                #[allow(missing_docs)]
                pub $field_name: $crate::reexports::bevy_ecs::change_detection::ResMut<'w, $cvar_ident>
            ),*
        }

        $(
            $crate::cvar!($(#[$cvar_doc])* $cvar_ident($cvar_path, $cvar_flags): $cvar_ty = $cvar_default);
        )*

        $(#[$plugin_doc])*
        #[derive(::std::default::Default)]
        $plugin_vis struct $cvar_collection_plugin;

        impl $crate::reexports::bevy_app::prelude::Plugin for $cvar_collection_plugin {
            fn build(&self, app: &mut $crate::reexports::bevy_app::prelude::App) {
                let mut management = app.world_mut().remove_resource::<$crate::CVarManagement>().unwrap();
                $(
                    app.register_type::<$cvar_ident>();
                    app.insert_resource::<$cvar_ident>($cvar_ident::default());
                    app.insert_resource::<$crate::CVarPeerData<$cvar_ident>>($crate::CVarPeerData::<$cvar_ident>::default());
                    management.register_cvar::<$cvar_ident>(app);
                    // Yes, these always run. I doubt it matters, but they do.
                    app.add_systems($crate::reexports::bevy_app::prelude::Last,
                        $crate::cvar_modified_system::<$cvar_ident>
                    );
                    if ($cvar_flags).contains($crate::CVarFlags::SAVED) {
                        let type_registry = app.world().resource::<$crate::reexports::bevy_ecs::prelude::AppTypeRegistry>().read();

                        ::std::assert!(
                            type_registry.get_type_data::<$crate::reexports::bevy_reflect::ReflectDeserialize>(::std::any::TypeId::of::<$cvar_ty>()).is_some(),
                            "CVar {} was registered as being a SAVED or MIRRORED cvar, but its value lacks reflection deserialization.",
                            stringify!($cvar_ident)
                        );

                        ::std::assert!(
                            type_registry.get_type_data::<$crate::reexports::bevy_reflect::ReflectSerialize>(::std::any::TypeId::of::<$cvar_ty>()).is_some(),
                            "CVar {} was registered as being a SAVED or MIRRORED cvar, but its value lacks reflection serialization.",
                            stringify!($cvar_ident)
                        );
                    }
                )*

                app.world_mut().insert_resource(management);
            }
        }
    };
}

/// Static meta information about a cvar, like its contained type and path.
pub trait CVarMeta: Resource + std::ops::Deref<Target = Self::Inner> {
    /// The inner type of the CVar.
    type Inner: std::fmt::Debug;
    /// The path of the CVar within the config.
    const CVAR_PATH: &'static str;
    /// The flags applied to this CVar.
    fn flags() -> CVarFlags;
}

// TODO: Implement when we have a network stack.
#[derive(Default, Resource)]
#[doc(hidden)]
pub struct CVarPeerData<T: CVarMeta> {
    _peers: HashMap<(), T>,
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

cvar_collection! {
    /// Collection of core CVars you can use as a system argument.
    pub struct CoreCVars & CoreCVarsMut {
        /// Enables logging ALL cvar modifications. This will log the change as info.
        log_cvar_changes = cvar LogCVarChanges("core.log_cvar_changes", CVarFlags::LOCAL | CVarFlags::RUNTIME): bool = false,
    }

    /// Plugin that handles registering all the core CVars.
    #[doc(hidden)]
    pub struct CoreCVarsPlugin;
}

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
