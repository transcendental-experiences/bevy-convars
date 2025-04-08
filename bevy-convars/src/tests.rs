use std::error::Error;

use bevy_app::App;

use crate::{
    CVarError, CVarFlags, CVarMeta, CVarsPlugin, cvar_collection,
    defaults::{IsDefault, IsDefaultMut},
};

const TEST_INTEGER_INIT_VAL: i32 = -5;

cvar_collection! {
    /// Collection of test CVars you can use as a system argument.
    pub struct TestCVars & TestCVarsMut {
        /// Test boolean flag.
        test_bool = cvar TestBool("testrig.test_bool", CVarFlags::RUNTIME): bool = true,

        /// Test numeric flag
        test_integer = cvar TestInteger("testrig.test_int", CVarFlags::RUNTIME): i32 = TEST_INTEGER_INIT_VAL,
    }

    /// Plugin that handles registering all the core CVars.
    #[doc(hidden)]
    pub struct TestCVarsPlugin;
}

#[must_use]
pub fn make_test_app() -> App {
    let mut app = App::new();
    app.add_plugins((CVarsPlugin, TestCVarsPlugin));
    app
}

#[test]
pub fn read_write_convar_direct() -> Result<(), Box<dyn Error>> {
    let mut app = make_test_app();
    let world = app.world_mut();

    assert_eq!(**world.resource::<TestInteger>(), TEST_INTEGER_INIT_VAL);

    **world.resource_mut::<TestInteger>() = 69;

    assert_eq!(**world.resource::<TestInteger>(), 69);

    assert!(**world.resource::<TestBool>());

    Ok(())
}

#[test]
pub fn read_write_default_convar_direct() -> Result<(), Box<dyn Error>> {
    let mut app = make_test_app();
    let world = app.world_mut();

    assert_eq!(**world.resource::<TestInteger>(), TEST_INTEGER_INIT_VAL);

    assert!(world.resource_ref::<TestInteger>().is_default());

    **world.resource_mut::<TestInteger>() = 69;

    assert_eq!(**world.resource::<TestInteger>(), 69);
    assert!(!world.resource_ref::<TestInteger>().is_default());

    world.resource_mut::<TestInteger>().reset_to_default();

    assert_eq!(**world.resource::<TestInteger>(), TEST_INTEGER_INIT_VAL);
    assert!(world.resource_ref::<TestInteger>().is_default());

    Ok(())
}

#[test]
#[cfg(feature = "parse_cvars")]
pub fn write_convar_deserialize() -> Result<(), Box<dyn Error>> {
    use std::str::FromStr as _;
    use toml_edit::de::ValueDeserializer;

    use crate::WorldExtensions;

    let mut app = make_test_app();
    let world = app.world_mut();

    world.set_cvar_deserialize(TestInteger::CVAR_PATH, ValueDeserializer::from_str("37")?)?;

    assert_eq!(**world.resource::<TestInteger>(), 37);
    Ok(())
}

#[test]
#[cfg(feature = "parse_cvars")]
pub fn write_convar_deserialize_unknown_convar() -> Result<(), Box<dyn Error>> {
    use std::str::FromStr as _;
    use toml_edit::de::ValueDeserializer;

    use crate::WorldExtensions;

    let mut app = make_test_app();
    let world = app.world_mut();

    let e = world.set_cvar_deserialize("testrig.not_real", ValueDeserializer::from_str("37")?);

    assert!(
        matches!(e, Err(CVarError::UnknownCVar)),
        "{} failed to match UnknownCVar.",
        e.err().unwrap()
    );

    Ok(())
}

#[test]
#[cfg(feature = "parse_cvars")]
pub fn write_convar_deserialize_wrong_type() -> Result<(), Box<dyn Error>> {
    use std::str::FromStr as _;
    use toml_edit::de::ValueDeserializer;

    use crate::WorldExtensions;

    let mut app = make_test_app();
    let world = app.world_mut();

    let e = world.set_cvar_deserialize(
        TestInteger::CVAR_PATH,
        ValueDeserializer::from_str("\"awawa\"")?,
    );

    assert!(
        matches!(e, Err(CVarError::FailedDeserialize(_))),
        "{} failed to match FailedDeserialize.",
        e.err().unwrap()
    );

    Ok(())
}

#[test]
pub fn write_convar_reflect_wrong_type() -> Result<(), Box<dyn Error>> {
    use crate::WorldExtensions;

    let mut app = make_test_app();
    let world = app.world_mut();

    let e = world.set_cvar_reflect(TestInteger::CVAR_PATH, &"awawa".to_owned());

    assert!(
        matches!(e, Err(CVarError::FailedApply { .. })),
        "{} failed to match FailedDeserialize.",
        e.err().unwrap()
    );

    Ok(())
}

#[cfg(feature = "parse_cvars")]
#[test]
pub fn write_convar_override() -> Result<(), Box<dyn Error>> {
    use std::str::FromStr as _;

    use crate::{WorldExtensions, parse::CVarOverride};

    let mut app = make_test_app();
    let world = app.world_mut();

    world.set_cvar_with_override(&CVarOverride::from_str("testrig.test_int=37")?)?;

    assert_eq!(**world.resource::<TestInteger>(), 37);
    assert!(
        world.resource_ref::<TestInteger>().is_default(),
        "An override should look like the default value for a convar."
    );

    Ok(())
}

#[test]
#[should_panic(
    expected = "Attempted to insert a duplicate CVar. CVar in question is testrig.test_int"
)]
pub fn duplicate_cvar_registration() {
    cvar_collection! {
        /// Collection of test CVars you can use as a system argument.
        pub struct ErrornousCVars & ErrornousCVarsMut {
            /// Test numeric flag that should cause an error.
            test_integer_shadow = cvar TestInteger("testrig.test_int", CVarFlags::LOCAL): i32 = 69,
        }

        /// Plugin that handles registering all the core CVars.
        pub struct ErrornousCVarsPlugin;
    }

    let mut app = make_test_app();
    app.add_plugins(ErrornousCVarsPlugin);
}

#[test]
#[should_panic(
    expected = "Tried to insert leaf test_int into a terminating node. Is there a duplicate or overlap? CVar in question is testrig.test_int.shadowed"
)]
pub fn mixed_branch_and_leaf_cvar_registration() {
    cvar_collection! {
        /// Collection of test CVars you can use as a system argument.
        pub struct ErrornousCVars & ErrornousCVarsMut {
            /// Test numeric flag that should cause an error.
            test_integer_branch_shadow = cvar TestInteger("testrig.test_int.shadowed", CVarFlags::LOCAL): i32 = 69,
        }

        /// Plugin that handles registering all the core CVars.
        pub struct ErrornousCVarsPlugin;
    }

    let mut app = make_test_app();
    app.add_plugins(ErrornousCVarsPlugin);
}
