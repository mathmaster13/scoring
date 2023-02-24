use std::collections::{HashMap};
use std::fmt::{Display, Formatter};
use std::num::NonZeroU8;
use std::ops::Index;
use crate::{BeaconError, ConeRemovalError, FieldCoordinate, Match, MaybeInvalidJunction};
use crate::sealed::Sealed;
use crate::InternalAllianceInfo;
use RedRemoteJunction::*;
use crate::ConeRemovalError::{BeaconOnJunction, JunctionIsEmpty};
use crate::id::{Alliance, FtcTeamID, MatchIndex};
use crate::locations::Terminal;
use crate::MaybeInvalidJunction::Valid;

#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug, Hash)]
#[repr(u8)]
pub enum RemoteCircuitPattern {
    Pattern1 = 0, Pattern2, Pattern3, Pattern4, Pattern5, Pattern6
}

// // copied from the maplit crate
// macro_rules! hashset {
//     (@single $($x:tt)*) => (());
//     (@count $($rest:expr),*) => (<[()]>::len(&[$(hashset!(@single $rest)),*]));
//
//     ($($key:expr,)+) => { hashset!($($key),+) };
//     ($($key:expr),*) => {
//         {
//             let _cap = hashset!(@count $($key),*);
//             let mut _set = ::std::collections::HashSet::with_capacity(_cap);
//             $(
//                 let _ = _set.insert($key);
//             )*
//             _set
//         }
//     };
// }

const CIRCUIT_PATTERNS: [&[RedRemoteJunction]; 6] = [
    &[Z2, Z3, Z4, Z5, Y5],
    &[X1, Y1, Z1, X2, Y3, X4, Y5],
    &[Z1, Y2, Z3, Y4, X5],
    &[Z1, Y2, Y3, Y4, X5],
    &[Y1, Z1, X2, Y3, X4, X5],
    &[X1, X2, X3, X4, X5, Y1, Y2, Y3, Y4, Y5, Z1, Z2, Z3, Z4, Z5]
];

#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug)]
#[repr(u8)]
// REPRESENTATION: [letter][number][junction points - 2]
// everything is zero-indexed
pub enum RedRemoteJunction {
    X1 = 0b000_000_00, X2 = 0b000_001_01, X3 = 0b000_010_00, X4 = 0b000_011_01, X5 = 0b000_100_00,
    Y1 = 0b001_000_01, Y2 = 0b001_001_10, Y3 = 0b001_010_11, Y4 = 0b001_011_10, Y5 = 0b001_100_01,
    Z1 = 0b010_000_00, Z2 = 0b010_001_01, Z3 = 0b010_010_00, Z4 = 0b010_011_01, Z5 = 0b010_100_00
}

crate::junction_impl!(RedRemoteJunction, 3, 5);

impl Display for RedRemoteJunction {
    #[inline(always)]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        <Self as std::fmt::Debug>::fmt(self, f)
    }
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug)]
#[repr(u8)]
// REPRESENTATION: [letter][number][junction points - 2]
// everything is zero-indexed
// THIS IS INTENTIONALLY BACKWARDS - it allows for red and blue to use the same logic, just with transmutes
pub enum BlueRemoteJunction {
    V1 = 0b010_000_00, V2 = 0b010_001_01, V3 = 0b010_010_00, V4 = 0b010_011_01, V5 = 0b010_100_00,
    W1 = 0b001_000_01, W2 = 0b001_001_10, W3 = 0b001_010_11, W4 = 0b001_011_10, W5 = 0b001_100_01,
    X1 = 0b000_000_00, X2 = 0b000_001_01, X3 = 0b000_010_00, X4 = 0b000_011_01, X5 = 0b000_100_00
}

crate::junction_impl!(BlueRemoteJunction, 3, 5);

// TODO see if space-optimizing this is worth it
struct InternalRemoteMatch {
    data: InternalAllianceInfo<RedRemoteJunction, 1>,
    circuit_pattern: RemoteCircuitPattern,
    junctions: HashMap<RedRemoteJunction, NonZeroU8>
}

impl Sealed for InternalRemoteMatch {}

impl Index<Alliance> for InternalRemoteMatch {
    type Output = [FtcTeamID];

    // TODO do we panic for blue or not
    fn index(&self, index: Alliance) -> &Self::Output {
        match index {
            Alliance::RED => &self.data.teams,
            Alliance::BLUE => &[]
        }
    }
}

impl Index<MatchIndex> for InternalRemoteMatch {
    type Output = FtcTeamID;

    fn index(&self, index: MatchIndex) -> &Self::Output {
        if index != MatchIndex::RED_CAPTAIN {
            panic!("Invalid index {index} for red remote match, which only has one red player.")
        }
        &self.data.teams[0]
    }
}

impl InternalRemoteMatch {
    fn add_cone(&mut self, alliance: Alliance, location: RedRemoteJunction) -> bool {
        if self.data.beacon_placements[0] != Valid(location) {
            match self.junctions.get_mut(&location) {
                Some(num) => {
                    match (*num).checked_add(1) {
                        Some(value) => {
                            self.junctions.insert(location, value);
                        }
                        None => panic!("The cone stack at {location} has overflown.")
                    }
                },
                None => {
                    self.junctions.insert(location, unsafe {
                        NonZeroU8::new_unchecked(1)
                    });
                }
            }
            true
        } else {
            false
        }
    }

    fn remove_cone(&mut self, location: RedRemoteJunction) -> Result<Alliance, ConeRemovalError> {
        if self.data.beacon_placements[0] != Valid(location) {
            match self.junctions.get_mut(&location) {
                Some(num) => {
                    let value = NonZeroU8::new((*num).get() - 1);
                    match value {
                        None => {
                            self.junctions.remove(&location);
                        }
                        Some(nonzero) => {
                            *num = nonzero;
                        }
                    }
                    Ok(Alliance::RED)
                }
                None => Err(JunctionIsEmpty)
            }
        } else {
            Err(BeaconOnJunction)
        }
    }

    fn add_terminal(&mut self, alliance: Alliance, terminal: Terminal) -> bool {
        let mut amounts = &mut self.data.terminal_amounts;
        let near_terminal = terminal == Terminal::Near;
        amounts[near_terminal as usize] += 1;
        near_terminal
    }

    fn add_beacon(&mut self, robot: MatchIndex, location: RedRemoteJunction) -> Result<(), BeaconError> {
        todo!()
    }

    fn penalty(&mut self, alliance: Alliance, points: u8) {
        todo!()
    }

    fn alliance_of(&self, robot: FtcTeamID) -> Option<Alliance> {
        todo!()
    }

    fn index_of(&self, robot: FtcTeamID) -> Option<MatchIndex> {
        todo!()
    }
}