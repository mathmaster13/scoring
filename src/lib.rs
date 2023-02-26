use std::mem::transmute;
use std::num::NonZeroU8;
use std::ops::Index;
use crate::locations::*;
use crate::id::*;
use bitvec::prelude::*;

#[cfg(test)]
mod tests;

mod traditional;
mod remote;
mod locations;
mod id;

// TODO IMPLEMENT DISPLAY FOR ALL APPLICABLE PUBLIC TYPES

// allows for abstraction over any field type
// (0, 0) is one coordinate of the field
// TODO decide public trait bounds on this type. Copy is sadly probably needed
pub trait FieldCoordinate: Ord + Copy + nohash::IsEnabled + sealed::Sealed {
    const ROWS: u8;
    const COLUMNS: u8;
    fn points(self) -> u8;
    fn row(self) -> u8;
    fn column(self) -> u8;
    fn coordinate(self) -> (u8, u8) {
        (self.row(), self.column())
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! junction_impl {
    ($struc:ty, $rows:literal, $columns:literal) => {
        impl Sealed for $struc {}
        impl nohash::IsEnabled for $struc {}

        // for safety with nohash
        impl std::hash::Hash for $struc {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                state.write_u8(*self as u8);
            }
        }

        impl FieldCoordinate for $struc {
            const ROWS: u8 = $rows;
            const COLUMNS: u8 = $columns;
            fn points(self) -> u8 {
                (self as u8 & 0b11) + 2
            }
            fn row(self) -> u8 {
                self as u8 >> 5
            }
            fn column(self) -> u8 {
                (self as u8 >> 2) & 0b111
            }
        }
    };
}

mod sealed {
    pub trait Sealed {}
}

// TODO beacon removal w/ match index of beacon that was removed
pub trait Match<T: FieldCoordinate>: sealed::Sealed + Index<Alliance, Output = [FtcTeamID]> + Index<MatchIndex, Output = FtcTeamID> {
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
pub trait Auto<T: FieldCoordinate, const R: usize, const B: usize>: Match<T> {
    type TeleOpType: TeleOp<T, R, B>; // FIXME we can't make this extend From<Self> without making this no longer object safe
    fn park(&mut self, robot: MatchIndex, location: impl Into<ParkingLocation>);
    fn into_teleop(self) -> Self::TeleOpType;
}
pub trait TeleOp<T: FieldCoordinate, const R: usize, const B: usize>: Match<T> {
    type EndGameType: EndGame<T, R, B>;
    fn into_end_game(self) -> Self::EndGameType;
}
pub trait EndGame<T: FieldCoordinate, const R: usize, const B: usize>: Match<T> {
    fn park_in_terminal(&mut self, robot: MatchIndex);
    fn end_match(self) -> (AllianceInfo<R>, AllianceInfo<B>);
}

#[derive(Debug, Eq, PartialEq, Clone)]
struct ConeStack {
    data: BitArr!(for 64, in u8),
    top_idx: Option<NonZeroU8>,
    red_count: u8,
    blue_count: u8
}

#[inline(always)]
fn as_u8(n: Option<NonZeroU8>) -> u8 {
    // SAFETY: Memory layout is identical.
    unsafe { transmute(n) }
}

impl ConeStack {
    fn new(value: Alliance) -> ConeStack {
        let mut stack = ConeStack {
            data: BitArray::new([value as u8, 0, 0, 0, 0, 0, 0, 0]),
            top_idx: NonZeroU8::new(1),
            red_count: 0,
            blue_count: 0
        };
        stack.increment_count(value);
        stack
    }
    fn push(&mut self, value: Alliance) {
        let idx = self.top_idx.and_then(|i| i.checked_add(1));
        if idx.map_or(true, |idx| idx >= unsafe { NonZeroU8::new_unchecked(64) }) {
            panic!("The cone stack has overflown.")
        } else {
            self.increment_count(value);
            self.top_idx = idx;
            self.data.set(
                as_u8(idx) as usize,
                value.into_bool()
            );
        }
    }
    // the bool is if the stack is empty after this pop
    fn pop(&mut self) -> Option<Alliance> {
        self.top_cone().map(|top| {
            self.decrement_count(top);
            self.top_idx = NonZeroU8::new(as_u8(self.top_idx) - 1);
            top
        })
    }
    #[inline]
    fn top_cone(&self) -> Option<Alliance> {
        self.top_idx.map(|i|
            Alliance::from(self.data[i.get() as usize])
        )
    }
    #[inline]
    fn increment_count(&mut self, alliance: Alliance) {
        if alliance.is_blue() {
            self.blue_count += 1;
        } else {
            self.red_count += 1;
        }
    }
    #[inline]
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
    #[inline(always)]
    fn is_empty(&self) -> bool {
        self.top_idx.is_none()
    }
    // seems like it's better to just have counters
    // fn reds_blues(self) -> [u8; 2] {
    //     // blue is true as a boolean
    //     let blue_count = self.data[..self.top_idx as usize].iter().by_vals().filter(|b| *b).count() as u8;
    //     [self.top_idx - blue_count, blue_count]
    // }
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, Hash)]
#[repr(u8)]
pub enum BeaconError {
    JunctionIsCapped, BeaconPreviouslyScored
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, Hash)]
pub struct BeaconScoredOutsideEndgame;

#[derive(Eq, PartialEq, Copy, Clone, Debug, Hash)]
#[repr(u8)]
pub enum ConeRemovalError {
    JunctionIsEmpty, BeaconOnJunction
}

// possession is handled by the Match implementation
#[derive(Debug)]
struct InternalAllianceInfo<T: FieldCoordinate, const N: usize> {
    teams: [FtcTeamID; N],
    penalty_points: u16,
    auto_points: u16, // aka TBP1
    terminal_amounts: [u8; 2],
    beacon_placements: [MaybeInvalidJunction<T>; N],
    parking_locations: [Option<ParkingLocation>; N]
}

impl <T: FieldCoordinate, const N: usize> InternalAllianceInfo<T, N> {
    fn new(teams: [FtcTeamID; N]) -> Self {
        Self {
            teams,
            penalty_points: 0,
            auto_points: 0,
            terminal_amounts: [0; 2],
            beacon_placements: [MaybeInvalidJunction::None; N],
            parking_locations: [None; N]
        }
    }

    /// Scores only the terminals and parking in auto.
    /// (This is just shared logic between traditional and remote)
    fn score_auto_parking_terminals(&mut self, signal_sleeves: [bool; N], signal_zone: SignalZone) {
        self.auto_points += {
            self.terminal_amounts[0] +
                (0..2).map(|i|
                    match self.parking_locations[i] {
                        Some(loc) => {
                            if loc == signal_zone.into() {
                                (signal_sleeves[i] as u8 + 1) * 10
                            } else {
                                !loc.is_signal_zone() as u8 * 2
                            }
                        }
                        None => 0
                    }
                ).sum::<u8>()
        } as u16;
        self.parking_locations = [None; N];
    }
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
enum MaybeInvalidJunction<T: FieldCoordinate> {
    Valid(T), Invalid, None
}

// probably requires an rc of cell, or unsafe fuckery
// pub struct Robot<'a, T: Junction, const N: usize, M: Match<T, N>> {
//     robot_match: &'a UnsafeCell<M>,
//     id: FtcTeamID,
//     _no_send: PhantomData<*mut ()>
// }

