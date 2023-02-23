use std::fmt::{Display, Formatter};
use std::mem::transmute;
use std::num::NonZeroU8;
use crate::{Junction, Match};

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Alliance {
    RED = 0, BLUE = 1
}

impl Display for Alliance {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", *self)
    }
}

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
pub struct FtcTeamID(pub i32); // i64 because negative team numbers exist in test matches

// TODO do we optimize Option<MatchIndex> by making this nonzero and moving the index over by 1?
#[derive(Eq, PartialEq, Copy, Clone, Debug, Hash)]
pub struct MatchIndex(u8);

macro_rules! match_index {
    ($index:expr, $alliance:expr) => {
        MatchIndex(($index << 1) + $alliance as u8)
    };
}

impl MatchIndex {
    /// Creates a new MatchIndex, panicking if the index is not valid.
    pub fn new<T: Junction, const R: usize, const B: usize>(robot_match: impl Match<T, R, B>, alliance: Alliance, index: u8) -> MatchIndex {
        let len = robot_match[alliance].len();
        if index as usize >= len {
            panic!("Attempt to create a MatchIndex for index {}, but {} only has {} robot(s) in this match.", index, alliance, len)
        }
        match_index!(index, alliance)
    }
    /// Creates a new MatchIndex, returning None if the index is not valid.
    pub fn try_new<T: Junction, const R: usize, const B: usize>(robot_match: impl Match<T, R, B>, alliance: Alliance, index: u8) -> Option<MatchIndex> {
        let len = robot_match[alliance].len();
        if index as usize >= len {
            None
        } else {
            Some(match_index!(index, alliance))
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

// // maybe this will be public but probably not
// pub(crate) trait AllianceIdentifier {
//     type Error;
//
//     fn into_alliance<T: Junction, const N: usize>(self, robot_match: &impl Match<T, N>) -> Result<Alliance, Self::Error>;
// }
//
// impl AllianceIdentifier for Alliance {
//     type Error = Infallible;
//
//     #[inline(always)]
//     fn into_alliance<T: Junction, const N: usize>(self, _: &impl Match<T, N>) -> Result<Alliance, Self::Error> {
//         Ok(self)
//     }
// }
//
// // A trait for items that can be converted fallibly to a MatchIndex given a match.
// pub(crate) trait RobotIdentifier: AllianceIdentifier {
//     type IndexConversionError;
//     fn into_match_index<T: Junction, const N: usize>(self, robot_match: &impl Match<T, N>) -> Result<MatchIndex, Self::IndexConversionError>;
// }
//
// impl AllianceIdentifier for MatchIndex {
//     type Error = Infallible;
//
//     #[inline(always)]
//     fn into_alliance<T: Junction, const N: usize>(self, _: &impl Match<T, N>) -> Result<Alliance, Self::Error> {
//         Ok(self.alliance())
//     }
// }
//
// impl RobotIdentifier for MatchIndex {
//     type IndexConversionError = Infallible;
//
//     #[inline(always)]
//     fn into_match_index<T: Junction, const N: usize>(self, _: &impl Match<T, N>) -> Result<MatchIndex, Self::IndexConversionError> {
//         Ok(self)
//     }
// }
//
// #[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug, Hash, Default)]
// struct RobotNotInMatch;
//
// impl AllianceIdentifier for FtcTeamID {
//     type Error = RobotNotInMatch;
//
//     fn into_alliance<T: Junction, const N: usize>(self, robot_match: &impl Match<T, N>) -> Result<Alliance, Self::Error> {
//         robot_match.alliance_of(self).ok_or(RobotNotInMatch)
//     }
// }
//
// impl RobotIdentifier for FtcTeamID {
//     type IndexConversionError = RobotNotInMatch;
//
//     fn into_match_index<T: Junction, const N: usize>(self, robot_match: &impl Match<T, N>) -> Result<MatchIndex, Self::IndexConversionError> {
//         robot_match.index_of(self).ok_or(RobotNotInMatch)
//     }
// }
