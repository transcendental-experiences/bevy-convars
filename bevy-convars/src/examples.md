# Examples
## Create and use CVars
```
# #![allow(dead_code)]
# use bevy_ecs::prelude::*;
# use bevy_app::prelude::*;
# use bevy_convars::*;
# use serde;
#
# // Dummies to mock the original code.
# #[derive(Copy, Clone, serde::Deserialize, serde::Serialize, bevy_reflect::Reflect, Debug)] pub enum SsaoQuality { High }
# #[derive(Copy, Clone, serde::Deserialize, serde::Serialize, bevy_reflect::Reflect, Debug)] pub enum MsaaSamplingConfig { Msaa4 }
# #[derive(Copy, Clone, serde::Deserialize, serde::Serialize, bevy_reflect::Reflect, Debug)] pub enum FxaaSensitivity { Medium }
# #[derive(Copy, Clone, serde::Deserialize, serde::Serialize, bevy_reflect::Reflect, Debug)] pub enum AntialiasMethod { Fxaa }
#
bevy_convars::cvar_collection! {
    pub struct RenderCVars & RenderCVarsMut {
        enable_xr = cvar EnableXr("render.enable_xr", CVarFlags::SAVED): bool = false,
        enable_renderdoc = cvar EnableRenderdoc("render.enable_renderdoc", CVarFlags::LOCAL): bool = false,

        /*
        *   Anti-aliasing
        */
        aa_method = cvar AaMethod("render.aa.method", CVarFlags::SAVED | CVarFlags::RUNTIME): AntialiasMethod = AntialiasMethod::Fxaa,

        fxaa_sensitivity = cvar FxaaSensitivty("render.aa.fxaa_sensitivity", CVarFlags::SAVED | CVarFlags::RUNTIME): FxaaSensitivity = FxaaSensitivity::Medium,

        msaa_samples = cvar MsaaSamples("render.aa.msaa_samples", CVarFlags::SAVED | CVarFlags::RUNTIME): MsaaSamplingConfig = MsaaSamplingConfig::Msaa4,

        /*
        *   SSAO.
        */
        enable_ssao = cvar EnableSsao("render.ssao.enabled", CVarFlags::SAVED | CVarFlags::RUNTIME): bool = true,
        ssao_quality = cvar RenderSsaoQuality("render.ssao.quality", CVarFlags::SAVED | CVarFlags::RUNTIME): SsaoQuality = SsaoQuality::High,
        ssao_object_thickness = cvar SsaoObjectThickness("render.ssao.object_thickness", CVarFlags::SAVED | CVarFlags::RUNTIME): f32 = 0.25
    }

    pub struct RenderCVarsPlugin;
}


fn my_system(
   cvars: RenderCVars,
   enable_ssao: Res<EnableSsao>,
   mut commands: Commands,
) {
   // Can read directly out of the RenderCVars param..
   let aa_method = **cvars.aa_method;

   // or from a specific cvar resource.
   // All CVar types implement Deref and DerefMut for their inner type to make them easy to unpack and modify.
   let ssao_on = **enable_ssao;

   // ...
}
```

## Load configuration files
```no_run
# #[allow(unexpected_cfgs)]
# use bevy_ecs::prelude::*;
# use bevy_app::prelude::*;
# use bevy_convars::prelude::*;
# use std::path::PathBuf;
# let mut app = App::new();
# fn get_user_directory() -> PathBuf { unimplemented!() }
// Add all your CVar plugins first, then:

// Bring your own implementation, I recommend the `directories` crate.
let mut user_data_directory: PathBuf = get_user_directory();

// Consider making this file name itself a CVar so users can specify an
// override on the command line.
user_data_directory.push("user_config.toml");

let cvar_loader =
    CVarLoaderPluginBuilder::fancy()
        // Load dev tooling config if we have them enabled.
        .add_asset_layer_if(cfg!(feature = "dev_tools"), "dev_tools.toml")
        // And load the user's config file.
        .with_user_config_file(user_data_directory)
        .build();

// Add the plugin, loading all layers and user configuration in one go.
app.add_plugins(cvar_loader);
```

## Apply command-line overrides
```no_run
# #[allow(unexpected_cfgs)]
# use bevy_ecs::prelude::*;
# use bevy_app::prelude::*;
# use bevy_convars::prelude::*;
# let mut app = App::new();
// Through one means or another, get yourself a list of CVarOverride.
// CVarOverride implements FromStr, so most command-line parsing libraries
// like clap can do it for you.

let overrides: Vec<CVarOverride> = todo!();

let world = app.world_mut();

for cvar in overrides.iter() {
    world.set_cvar_with_override(cvar);
}
```