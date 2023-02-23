use std::mem::transmute;
use crate::Match;

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Alliance {
    RED = 0, BLUE = 1
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

// FIXME if we seal the Match trait, this can be optimized into a single u8
// FIXME where the first bit is the alliance and the others are the index
// FIXME if this happens, alliance and index should be private
#[derive(Eq, PartialEq, Copy, Clone, Debug, Hash)]
pub struct MatchIndex {
    pub(crate) alliance: Alliance,
    pub(crate) index: usize
}

impl MatchIndex {
    // infrastructure in case we seal Match
    #[inline(always)]
    pub fn new(alliance: Alliance, index: usize) -> MatchIndex {
        MatchIndex {
            alliance,
            index
        }
    }
    #[inline(always)]
    pub fn alliance(self) -> Alliance {
        self.alliance
    }
    #[inline(always)]
    pub fn index(self) -> usize {
        self.index
    }
    #[inline(always)]
    pub fn is_captain(self) -> bool {
        self.index == 0
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
