use std::str::FromStr;

use thiserror::Error;

/// A partially parsed CVar override. This ensures its in the correct format, but does not ensure it'll deserialize!
#[derive(Clone, Debug)]
pub struct CVarOverride(pub(crate) String, pub(crate) toml_edit::Value);

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum CVarOverrideParseError {
    #[error("Not a valid override, a CVar override must have a CVar path (a.b.c) on the left.")]
    InvalidPath,
    #[error("Not a valid override, a CVar override must have TOML on the right.")]
    InvalidToml,
    #[error("Not a valid override, a CVar override must be of form `left=right`")]
    DoesntLookLikeAnOverride,
}

impl TryFrom<&str> for CVarOverride {
    type Error = CVarOverrideParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let (left, right) = value
            .split_once('=')
            .ok_or(CVarOverrideParseError::DoesntLookLikeAnOverride)?;

        let value =
            toml_edit::Value::from_str(right).map_err(|_| CVarOverrideParseError::InvalidToml)?;

        Ok(CVarOverride(left.to_owned(), value))
    }
}

impl FromStr for CVarOverride {
    type Err = CVarOverrideParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}
