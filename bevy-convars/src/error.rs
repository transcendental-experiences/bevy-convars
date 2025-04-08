use bevy_ecs::world::error::ResourceFetchError;
use bevy_reflect::ApplyError;

/// Errors that can occur when manipulating CVars.
#[derive(thiserror::Error, Debug)]
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
    /// Error indicating the CVar type is missing a [ComponentId](bevy_ecs::component::ComponentId) and is likely not registered correctly.
    #[error("Missing ComponentID, was the resource registered?")]
    MissingCid,
    /// Error indicating the underlying type of the CVar cannot be deserialized, and as such cannot be reflected over.
    #[error("Underlying CVar type cannot be deserialized.")]
    CannotDeserialize,
    /// Error indicating the CVar failed to deserialize.
    #[error("Failed to deserialize.")]
    FailedDeserialize(String),
    /// Error indicating applying a value to the CVar failed, containing the inner error.
    #[error("Failed to apply value to CVar. ({inner:?})")]
    FailedApply {
        /// The inner error.
        inner: ApplyError,
    },
    /// Error indicating that the world could not fulfill the requested operation due to an access conflict with an ongoing operation.
    #[error("The requested operation conflicts with another ongoing operation on the world and cannot be performed.")]
    AccessConflict,
}

impl From<ApplyError> for CVarError {
    fn from(value: ApplyError) -> Self {
        Self::FailedApply { inner: value }
    }
}

impl From<ResourceFetchError> for CVarError {
    fn from(value: ResourceFetchError) -> Self {
        match value {
            ResourceFetchError::NotRegistered => CVarError::UnknownCVar,
            ResourceFetchError::DoesNotExist(_) => CVarError::UnknownCVar,
            ResourceFetchError::NoResourceAccess(_) => CVarError::AccessConflict,
        }
    }
}