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
        #[reflect(Default__CALL_CVARDECLIMPORTS, Resource__CALL_CVARDECLIMPORTS, CVar__MACRO_JANK)]
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

        impl $crate::reflect::CVarMeta for $cvar_ident {
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
/// # use bevy_convars::*;
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
                #[allow(dead_code)]
                pub $field_name: $crate::reexports::bevy_ecs::change_detection::Res<'w, $cvar_ident>
            ),*
        }

        $(#[$collection_doc])*
        #[derive($crate::reexports::bevy_ecs::system::SystemParam)]
        $collection_vis struct $cvar_collection_ident_mut<'w> {
            $(
                #[allow(missing_docs)]
                #[allow(dead_code)]
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
