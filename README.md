# Bevy Convars
![Crates.io Version](https://img.shields.io/crates/v/bevy-convars) ![docs.rs](https://img.shields.io/docsrs/bevy-convars)

This is a crate that provides an implementation of Convars (console variables or config variables depending on who you ask), provided as bevy resources, with support for serialization, change detection, and the works.
Convars are presented as resources within the Bevy world, and can be accessed as such without any special code.

```rust
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

## State of Development
This crate is a bit early, and while it likely can replace your existing solution it needs some elbow grease to use outside of the supported use cases.

Contributions are welcome!

- [x] Config loading.
  - [x] Layered configs loading.
  - [ ] Builtin system for config presets (can easily be implemented by loading config files containing the preset.)
  - [ ] Support for alternate, non-TOML formats and mediums (like SQLite)
- [x] Config saving.
  - [x] File format preserving saving. (i.e. not modifying comments/etc)
  - [x] Aware of the difference between user set and default values, even if the values are equal.
  - [ ] Optional automatic sync to disk for user config.
  - [ ] Support for alternate, non-TOML formats and mediums (like SQLite)
- [x] Config reflection.
- [x] Intelligent default value handling.
- [ ] Built-in support for existing netcode libraries.
  - [ ] [bevy_replicon](https://github.com/projectharmonia/bevy_replicon)
  - [ ] [lightyear](https://github.com/cBournhonesque/lightyear)
- [ ] Full no_std support. (Needs further testing and work)
  - [ ] WASM support.
- [x] Minimal set of required Bevy features.
- [ ] Doesn't require config options to be serializable.
- [ ] Fully panic safe.

## Bevy Compatibility
This library tracks Bevy's releases, at this time the following holds:
| Bevy Version | bevy-convars Version |
| ------------ | -------------------- |
| 0.16.0-rc.4  | 0.1.0                |

## License
This work is available under EITHER the Apache 2.0 license (as provided in LICENSE-APACHE) or the MIT license (as provided in LICENSE-MIT).