use std::error::Error;

use toml_edit::ImDocument;

use crate::{reflect::CVarMeta, tests::{make_test_app, TestArray, TestInteger}, CVarManagement};

use super::{cvar_doc::{CVarDocScanner, DocumentContext}, ConfigLoader};

const TEST_DOCUMENT: &str = include_str!("test_document.toml");

#[test]
pub fn parse_test_document() {
    let app = make_test_app();

    let document = ImDocument::parse(TEST_DOCUMENT).unwrap();
    let document = DocumentContext::new(document, "test_document.toml".to_string());

    let scanner = CVarDocScanner::new(document);

    let cvars = scanner.find_cvars(app.world().resource::<CVarManagement>());

    println!("{:?}", cvars);

    assert!(cvars.iter().any(|(key, _)| *key == TestArray::CVAR_PATH))
}

#[test]
pub fn apply_test_document() -> Result<(), Box<dyn Error>> {
    let mut app = make_test_app();

    let document = ImDocument::parse(TEST_DOCUMENT).unwrap();
    let document = DocumentContext::new(document, "test_document.toml".to_string());

    let loader = ConfigLoader::default();

    let world = app.world_mut();

    loader.apply(world, document)?;

    assert_eq!(**world.resource::<TestInteger>(), 4);

    assert_eq!(**world.resource::<TestArray>(), [1, 2, -3]);


    Ok(())
}