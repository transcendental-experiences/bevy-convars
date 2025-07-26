//! Contains tools for embedding config layers into the game binary.

/// Adds an embedded config layer to the world.
/// This loads and applies the file at `path` using [include_str!].
/// ```no_test
///     add_embedded_layer!(app, "config/debug.toml");
/// ```
#[macro_export]
macro_rules! add_embedded_layer {
    ($app:ident, $path:literal) => {
        let file = ::std::include_str!($path);
        let loader = $crate::loader::ConfigLoader::default();

        loader.apply_from_string($app.world_mut(), file, Some($path), false).expect("Embedded config file failed to parse and apply.");
    };
}