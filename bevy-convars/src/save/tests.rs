use std::str::FromStr;

use toml_edit::DocumentMut;

use crate::{
    CVarError,
    tests::{self, TestBool, TestInteger},
};

#[test]
pub fn save_unmodified_world() -> Result<(), CVarError> {
    let app = tests::make_test_app();

    let mut save_ctx = crate::save::CVarSaveContext::blank();

    save_ctx.save_world(app.world())?;

    let result = save_ctx.return_document();

    assert_eq!(result.to_string(), "");

    Ok(())
}

const EXPECTED_1: &str = include_str!("expected_cfg_1.toml");

#[test]
pub fn save_modified_world() -> Result<(), CVarError> {
    let mut app = tests::make_test_app();

    {
        let world = app.world_mut();

        **world.resource_mut::<TestBool>() = true;
        **world.resource_mut::<TestInteger>() = 42;
    }

    let mut save_ctx = crate::save::CVarSaveContext::blank();

    save_ctx.save_world(app.world())?;

    let result = save_ctx.return_document();

    assert_eq!(result.to_string(), EXPECTED_1);

    Ok(())
}

#[test]
pub fn save_over_existing_cfg() -> Result<(), CVarError> {
    const INITIAL: &str = include_str!("existing_cfg_1.toml");
    const EXPECTED: &str = include_str!("expected_cfg_2.toml");
    let mut app = tests::make_test_app();

    {
        let world = app.world_mut();

        **world.resource_mut::<TestBool>() = true;
        **world.resource_mut::<TestInteger>() = 42;
    }

    let document = DocumentMut::from_str(INITIAL)?;

    let mut save_ctx = crate::save::CVarSaveContext::from_document(document);

    save_ctx.save_world(app.world())?;

    let result = save_ctx.return_document();

    assert_eq!(result.to_string(), EXPECTED);

    Ok(())
}
