//! Provides support for saving CVars to a TOML config file.

use bevy_ecs::{
    change_detection::MaybeLocation,
    component::Tick,
    reflect::{AppTypeRegistry, ReflectResource},
    world::{Ref, World},
};
use bevy_reflect::{Reflect, ReflectSerialize};
use serde::Serialize;
use toml_edit::{DocumentMut, Item, Table, ser::ValueSerializer};

use crate::{
    CVarError, CVarFlags, CVarManagement,
    reflect::{CVarMeta, ReflectCVar},
};

#[cfg(test)]
mod tests;

/// Provides a context for mutating a TOML document to save CVars to it.
///
/// # Example
/// ```no_run
/// # use bevy_ecs::prelude::*;
/// # use bevy_convars::save::*;
/// # let world = World::new();
/// let mut context = CVarSaveContext::blank();
///
/// // Let's save the world config to this.
///
/// context.save_world(&world);
///
/// // And serialize out the results so we can save it.
/// let file_contents = context.to_string();
/// ```
pub struct CVarSaveContext(DocumentMut);

impl CVarSaveContext {
    /// Creates a new context with an empty document.
    pub fn blank() -> Self {
        Self(DocumentMut::new())
    }

    /// Creates a new context with an existing document.
    pub fn from_document(doc: DocumentMut) -> Self {
        Self(doc)
    }

    /// Returns the document used from the context, destroying the context.
    pub fn return_document(self) -> DocumentMut {
        self.0
    }

    fn get_cvar_entry(&mut self, path: &str) -> Result<toml_edit::Entry<'_>, CVarError> {
        let sections = path.split('.');
        let section_count = sections.clone().count();
        let leading_sections = sections.clone().take(section_count - 1);
        let final_section = sections.last().unwrap();

        let mut cur_table = self.0.as_table_mut();

        for section in leading_sections {
            cur_table = cur_table
                .entry(section)
                .or_insert(toml_edit::Item::Table(Table::new()))
                .as_table_mut()
                .ok_or(CVarError::MalformedConfigDuringWrite("Expected a table."))?;
        }

        Ok(cur_table.entry(final_section))
    }

    /// Saves an individual CVar to the document.
    fn save_cvar_inner(&mut self, path: &str, value: &impl Serialize) -> Result<(), CVarError> {
        let entry = self.get_cvar_entry(path)?;

        *entry.or_insert(toml_edit::Item::None) =
            Item::Value(value.serialize(ValueSerializer::new())?);

        Ok(())
    }

    fn save_cvar_inner_erased(
        &mut self,
        path: &str,
        value: &bevy_reflect::serde::Serializable,
    ) -> Result<(), CVarError> {
        let entry = self.get_cvar_entry(path)?;

        *entry.or_insert(toml_edit::Item::None) =
            Item::Value(value.serialize(ValueSerializer::new())?);

        Ok(())
    }

    /// Manually save an individual CVar to the document.
    /// # Remarks
    /// This does not check for the presence of [CVarFlags::SAVED], and as such can be used to specially handle some CVars.
    pub fn save_cvar<T: CVarMeta>(&mut self, cvar: &T) -> Result<(), CVarError>
    where
        T::Inner: Serialize,
    {
        self.save_cvar_inner(T::CVAR_PATH, &**cvar)
    }

    /// Manually save an individual CVar to the document, from the world.
    /// # Remarks
    /// This does not check for the presence of [CVarFlags::SAVED], and as such can be used to specially handle some CVars.
    pub fn save_cvar_from_world<T: CVarMeta>(&mut self, world: &World) -> Result<(), CVarError>
    where
        T::Inner: Serialize,
    {
        self.save_cvar_inner(T::CVAR_PATH, &**world.resource::<T>())
    }

    /// Saves a world's CVars to the document.
    /// # Remarks
    /// This obeys [CVarFlags::SAVED] and will not attempt to save CVars without it.
    pub fn save_world(&mut self, world: &World) -> Result<(), CVarError> {
        let management: &CVarManagement = world.resource::<CVarManagement>();
        let registry = world.resource::<AppTypeRegistry>().read();
        let types = management.iterate_cvar_types();

        for reg in types {
            let cvar = reg.data::<ReflectCVar>().expect("Impossible.");

            if !cvar.flags().contains(CVarFlags::SAVED) {
                continue;
            }

            let Some(serialize) = registry.get_type_data::<ReflectSerialize>(cvar.inner_type())
            else {
                panic!(
                    "Can't save a saveable cvar due to lack of ReflectSerialize implementation. CVar in question is {}",
                    cvar.cvar_path()
                );
            };

            let resource = reg.data::<ReflectResource>().expect("Impossible.");

            let cvar_id = management.tree.get(cvar.cvar_path()).unwrap();

            let change_data = world.get_resource_change_ticks_by_id(cvar_id).unwrap();

            let caller = MaybeLocation::caller();

            let res = resource.reflect(world)?;
            let resource: Ref<dyn Reflect> = {
                // Jank, Bevy is missing an API for this..

                Ref::new(
                    res,
                    &change_data.added,
                    &change_data.changed,
                    Tick::new(0),
                    Tick::new(0),
                    caller.as_ref(),
                )
            };

            if cvar.is_default_value(resource) {
                continue;
            }

            self.save_cvar_inner_erased(
                cvar.cvar_path(),
                &serialize.get_serializable(
                    cvar.reflect_inner(res.as_partial_reflect())?
                        .try_as_reflect()
                        .unwrap(),
                ),
            )?;
        }

        Ok(())
    }
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for CVarSaveContext {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}
