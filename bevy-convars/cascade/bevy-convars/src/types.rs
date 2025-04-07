
use bevy_reflect::Reflect;
use std::ops;

/// Bevy cannot reflect over bitflags!, so we do it the old fashioned way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
pub struct CVarFlags(u32);

impl CVarFlags {
    /// The no-op/default flag set.
    pub const LOCAL: CVarFlags = CVarFlags(0);
    /// Indicates this cvar should be saved to disk as part of the user's settings.
    pub const SAVED: CVarFlags = CVarFlags(0b0000_0001);
    /// Indicates this cvar is for mirrored to and from peers for replication. Peers will know the value of this CVar for this client.
    pub const MIRRORED: CVarFlags = CVarFlags(0b0000_0010);
    /// Indicates this cvar is replicated from the cloud service if available.
    pub const FROM_CLOUD: CVarFlags = CVarFlags(0b0000_0100);
    /// Indicates this cvar is respected at runtime if modified. This is a hint of intent!
    /// CVars without this flag set should warn the user to restart the game.
    pub const RUNTIME: CVarFlags = CVarFlags(0b0000_1000);
}

impl ops::BitOr for CVarFlags {
    type Output = CVarFlags;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl ops::BitAnd for CVarFlags {
    type Output = CVarFlags;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl CVarFlags {
    pub fn contains(&self, other: CVarFlags) -> bool {
        let and = *self & other;

        and.0 == other.0
    }
}
