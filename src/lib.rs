use std::ops::Index;
use crate::locations::*;
use crate::id::*;
use bitvec::prelude::*;

#[cfg(test)]
mod tests;

mod traditional;
mod locations;
mod id;

// TODO decide what is public/sealed

// allows for abstraction over any field type
// (0, 0) is one coordinate of the field
pub trait Junction: Ord + Copy { // FIXME why bound on copy
    const ROWS: u8;
    const COLUMNS: u8;
    fn points(self) -> u8;
    fn row(self) -> u8;
    fn column(self) -> u8;
    fn coordinate(self) -> (u8, u8) {
        (self.row(), self.column())
    }
}

// TODO index bounds may or may not work with remote; slices instead of arrays may be needed
// TODO should everything take in a MatchIndex for consistency?
// TODO ok sealing Match sounds great. also removing the const N limitation
pub trait Match<T: Junction, const N: usize>: Index<Alliance, Output = [FtcTeamID; N]>
    + Index<MatchIndex, Output = FtcTeamID> {
    fn add_cone(&mut self, alliance: Alliance, location: T) -> bool;
    type ConeRemovalErrorType;
    fn remove_cone(&mut self, location: T) -> Result<Alliance, Self::ConeRemovalErrorType>;
    fn add_terminal(&mut self, alliance: Alliance, terminal: Terminal) -> bool;
    type BeaconErrorType;
    fn add_beacon(&mut self, robot: MatchIndex, location: T) -> Result<(), Self::BeaconErrorType>;
    fn penalty(&mut self, alliance: Alliance, points: u8);
    fn alliance_of(&self, robot: FtcTeamID) -> Option<Alliance> {
        self.index_of(robot).map(|i| i.alliance())
    }
    fn index_of(&self, robot: FtcTeamID) -> Option<MatchIndex>;
}
pub trait Auto<T: Junction, const N: usize>: Match<T, N> {
    type TeleOpType: TeleOp<T, N>;
    fn park(&mut self, robot: MatchIndex, location: impl Into<ParkingLocation>) -> bool;
    fn into_teleop(self) -> Self::TeleOpType;
}
pub trait TeleOp<T: Junction, const N: usize>: Match<T, N> {
    type EndGameType: EndGame<T, N>;
    fn into_end_game(self) -> Self::EndGameType;
}
pub trait EndGame<T: Junction, const N: usize>: Match<T, N> {
    fn park_in_terminal(&mut self, robot: MatchIndex) -> bool;
    fn end_match(self) -> [AllianceInfo<N>; 2];
}

#[derive(Debug, Eq, PartialEq, Clone)]
struct ConeStack {
    data: BitArr!(for 64, in u8),
    top_idx: u8,
    red_count: u8,
    blue_count: u8,
    /// to optimize the memory layout of Option<ConeStack>
    _magic: bool
}

impl ConeStack {
    fn new(value: Alliance) -> ConeStack {
        let mut stack = ConeStack {
            data: BitArray::new([value as u8, 0, 0, 0, 0, 0, 0, 0]),
            top_idx: 0,
            red_count: 0,
            blue_count: 0,
            _magic: false
        };
        stack.increment_count(value);
        stack
    }
    fn push(&mut self, value: Alliance) {
        let new_idx = self.top_idx + 1;
        if new_idx >= 64 {
            panic!("The cone stack has overflown.")
        }
        self.increment_count(value);
        self.top_idx = new_idx;
        self.data.set(new_idx as usize, value.into_bool());
    }
    // the bool is if the stack is empty after this pop
    fn pop(&mut self) -> (Alliance, bool) {
        let top = self.top_cone();
        (top, {
            let is_empty = self.top_idx == 0;
            self.decrement_count(top);
            if !is_empty {
                self.top_idx -= 1;
            }
            is_empty
        })
    }
    #[inline]
    fn top_cone(&self) -> Alliance {
        Alliance::from(self.data[self.top_idx as usize])
    }
    #[inline]
    fn increment_count(&mut self, alliance: Alliance) {
        if alliance.is_blue() {
            self.blue_count += 1;
        } else {
            self.red_count += 1;
        }
    }

    fn decrement_count(&mut self, alliance: Alliance) {
        if alliance.is_blue() {
            self.blue_count -= 1;
        } else {
            self.red_count -= 1;
        }
    }
    #[inline]
    fn count(&self, alliance: Alliance) -> u8 {
        if alliance.is_blue() {
            self.blue_count
        } else {
            self.red_count
        }
    }
    // seems like it's better to just have counters
    // fn reds_blues(self) -> [u8; 2] {
    //     // blue is true as a boolean
    //     let blue_count = self.data[..self.top_idx as usize].iter().by_vals().filter(|b| *b).count() as u8;
    //     [self.top_idx + 1 - blue_count, blue_count]
    // }
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, Hash)]
#[repr(u8)]
pub enum BeaconError {
    JunctionIsCapped, BeaconPreviouslyScored, ScoredOutsideEndgame
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, Hash)]
#[repr(u8)]
pub enum ConeRemovalError {
    JunctionIsEmpty, BeaconOnJunction
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub struct AllianceInfo<const N: usize> {
    pub alliance: Alliance,
    pub teams: [FtcTeamID; N],
    pub penalty_points: u16,
    pub auto_points: u16,
    pub teleop_points: u16,
    pub endgame_points: u16,
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, Hash)]
enum MaybeInvalidJunction<T: Junction> {
    Valid(T), Invalid, None
}

// probably requires an rc of cell, or unsafe fuckery
// pub struct Robot<'a, T: Junction, const N: usize, M: Match<T, N>> {
//     robot_match: &'a UnsafeCell<M>,
//     id: FtcTeamID,
//     _no_send: PhantomData<*mut ()>
// }

