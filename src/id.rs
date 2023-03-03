//! The module for structures representing teams or alliances.
//! [`FtcTeamID`] represents a team as they appear on a match leaderboard,
//! while [`MatchIndex`] represents a team's role in a specific match.
//! Prefer using [`MatchIndex`] if possible,
//! since it does not require checking if a given ID is in the match.
use crate::{FieldCoordinate, Match};
use std::fmt;
use std::fmt::{Display, Formatter};
use std::mem::transmute;

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Alliance {
    RED = 0, BLUE = 1 // TODO rename to Red, Blue?
}
crate::display_impl_as_debug!(Alliance);

impl Alliance {
    // these are not public so they are not trait implementations
    #[inline(always)]
    pub(crate) fn into_bool(self) -> bool {
        unsafe { transmute(self) }
    }
    #[inline(always)]
    // different name for semantics
    pub(crate) fn is_blue(self) -> bool {
        self.into_bool()
    }
    #[inline(always)]
    pub(crate) fn from(value: bool) -> Alliance {
        unsafe { transmute(value) }
    }
}

#[repr(transparent)]
#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug, Hash)]
// TODO rename to FtcTeamId?
pub struct FtcTeamID(pub i32); // i32 because negative team numbers exist in test matches
crate::display_impl_as_debug!(FtcTeamID);

// TODO what if this just. was normal
// TODO and if it isn't normal, do this...
// #[rustc_layout_scalar_valid_range_end(254)]
#[derive(Eq, PartialEq, Copy, Clone, Debug, Hash)]
pub struct MatchIndex(pub(crate) u8);

macro_rules! match_index {
    ($alliance:expr, $index:expr) => {
        MatchIndex(($index << 1) + $alliance as u8)
    };
}

impl MatchIndex {
    pub const RED_CAPTAIN: MatchIndex = match_index!(Alliance::RED, 0); // 0
    pub const BLUE_CAPTAIN: MatchIndex = match_index!(Alliance::BLUE, 0); // 1
    pub const RED_FIRST_PICK: MatchIndex = match_index!(Alliance::RED, 1); // 2
    pub const BLUE_FIRST_PICK: MatchIndex = match_index!(Alliance::BLUE, 1); // 3

    #[inline(always)]
    /// Creates a new MatchIndex.
    pub fn new(alliance: Alliance, index: u8) -> Self {
        match_index!(alliance, index)
    }

    /// Creates a new MatchIndex, panicking if the index is not valid.
    pub fn for_match<T: FieldCoordinate>(
        robot_match: &impl Match<T>,
        alliance: Alliance,
        index: u8,
    ) -> Self {
        let len = robot_match[alliance].len();
        if index as usize >= len {
            panic!("Attempt to create a MatchIndex for index {}, but {} only has {} robot(s) in this match.", index, alliance, len)
        }
        match_index!(alliance, index)
    }
    /// Creates a new MatchIndex, returning None if the index is not valid.
    pub fn try_for_match<T: FieldCoordinate>(
        robot_match: &impl Match<T>,
        alliance: Alliance,
        index: u8,
    ) -> Option<Self> {
        let len = robot_match[alliance].len();
        if index as usize >= len {
            None
        } else {
            Some(match_index!(alliance, index))
        }
    }
    #[inline]
    pub fn alliance(self) -> Alliance {
        unsafe { transmute(self.0 & 1) }
    }
    #[inline]
    pub fn index(self) -> usize {
        (self.0 >> 1) as usize
    }
    #[inline]
    pub fn is_captain(self) -> bool {
        self.0 >> 1 == 0
    }
}

impl Display for MatchIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("MatchIndex")
            .field("alliance", &self.alliance())
            .field("index", &self.index())
            .finish()
    }
}
