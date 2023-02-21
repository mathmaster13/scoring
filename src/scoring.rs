use std::collections::HashMap;
use std::num::NonZeroU8;
use std::ops::{Index, IndexMut};
use std::mem::transmute;
use crate::scoring::BeaconError::{BeaconPreviouslyScored, RobotNotInMatch, JunctionIsCapped};
use crate::scoring::MaybeInvalidJunction::Valid;

// TODO decide what is public - index traits + alliance info?
// TODO index and alliance or FTC ID?

#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug, Hash)]
#[repr(u8)]
// REPRESENTATION: [letter][number][junction points - 2]
// everything is zero-indexed
pub enum TraditionalJunction {
    V1 = 0b000_000_00, V2 = 0b000_001_01, V3 = 0b000_010_00, V4 = 0b000_011_01, V5 = 0b000_100_00,
    W1 = 0b001_000_01, W2 = 0b001_001_10, W3 = 0b001_010_11, W4 = 0b001_011_10, W5 = 0b001_100_01,
    X1 = 0b010_000_00, X2 = 0b010_001_11, X3 = 0b010_010_00, X4 = 0b010_011_11, X5 = 0b010_100_00,
    Y1 = 0b011_000_01, Y2 = 0b011_001_10, Y3 = 0b011_010_11, Y4 = 0b011_011_10, Y5 = 0b011_100_01,
    Z1 = 0b100_000_00, Z2 = 0b100_001_01, Z3 = 0b100_010_00, Z4 = 0b100_011_01, Z5 = 0b100_100_00
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, Hash)]
#[repr(u8)]
pub enum ParkingLocation {
    LeftSignalZone = 0b0100_0000, MiddleSignalZone, RightSignalZone,
    NearTerminal = 0b1000_0000, FarTerminal,
    Substation = 0b0010_0000
}

impl ParkingLocation {
    #[inline(always)]
    fn is_signal_zone(self) -> bool {
        self as u8 & 0b0100_0000 != 0
    }

    #[inline(always)]
    fn is_terminal(self) -> bool {
        self as u8 & 0b1000_0000 != 0
    }
}

impl From<Terminal> for ParkingLocation {
    fn from(value: Terminal) -> Self {
        unsafe { transmute(value) }
    }
}

impl From<SignalZone> for ParkingLocation {
    fn from(value: SignalZone) -> Self {
        unsafe { transmute(value) }
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, Hash)]
#[repr(u8)]
pub enum Terminal {
    Near = 0b1000_0000, Far
}

impl TryFrom<ParkingLocation> for Terminal {
    type Error = ();

    fn try_from(value: ParkingLocation) -> Result<Self, Self::Error> {
        if value.is_terminal() {
            Ok(unsafe { transmute(value) })
        } else {
            Err(())
        }
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, Hash)]
#[repr(u8)]
pub enum SignalZone {
    Left = 0b0100_0000, Middle, Right
}

impl TryFrom<ParkingLocation> for SignalZone {
    type Error = ();

    fn try_from(value: ParkingLocation) -> Result<Self, Self::Error> {
        if value.is_signal_zone() {
            Ok(unsafe { transmute(value) })
        } else {
            Err(())
        }
    }
}

impl Junction for TraditionalJunction {
    const ROWS: u8 = 5;
    const COLUMNS: u8 = 5;
    fn points(&self) -> u8 {
        (*self as u8 & 0b11) + 2
    }
    fn row(&self) -> u8 {
        *self as u8 >> 5
    }
    fn column(&self) -> u8 {
        (*self as u8 >> 2) & 0b111
    }
}

// allows for abstraction over any field type
// (0, 0) is one coordinate of the field
pub trait Junction: Ord + Copy {
    const ROWS: u8;
    const COLUMNS: u8;
    fn points(&self) -> u8;
    fn row(&self) -> u8;
    fn column(&self) -> u8;
    fn coordinate(&self) -> (u8, u8) {
        (self.row(), self.column())
    }
}

#[repr(transparent)]
#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug, Hash)]
pub struct FtcTeamID(pub i32); // i64 because negative team numbers exist in test matches

pub trait Match<T: Junction, const N: usize> {
    fn add_cone(&mut self, alliance: Alliance, location: TraditionalJunction) -> bool;
    fn add_terminal(&mut self, alliance: Alliance, terminal: Terminal) -> bool;
    type BeaconErrorType;
    fn add_beacon(&mut self, robot: FtcTeamID, location: TraditionalJunction) -> Result<(), Self::BeaconErrorType>;
    fn penalty(&mut self, alliance: Alliance, points: u8);
    fn alliance_of(&self, robot: FtcTeamID) -> Option<Alliance>;
    // fn index_of(&self, robot: FtcTeamID) -> Option<usize>;
}
pub trait Auto<T: Junction, const N: usize>: Match<T, N> {
    type TeleOpType: TeleOp<T, N>;
    fn new(red: [FtcTeamID; N], blue: [FtcTeamID; N]) -> Self
    where Self: Sized {
        let option = Self::try_new(red, blue);
        if let Some(robot_match) = option {
            robot_match
        } else {
            panic!("A problem occurred while trying to construct a match.")
        }
    }
    fn try_new(red: [FtcTeamID; N], blue: [FtcTeamID; N]) -> Option<Self>
    where Self: Sized;
    fn park(&mut self, robot: FtcTeamID, location: ParkingLocation) -> bool;
    fn into_teleop(self) -> Self::TeleOpType;
}
pub trait TeleOp<T: Junction, const N: usize>: Match<T, N> {
    type EndGameType: EndGame<T, N>;
    fn into_end_game(self) -> Self::EndGameType;
}
pub trait EndGame<T: Junction, const N: usize>: Match<T, N> {
    fn park_in_terminal(robot: FtcTeamID) -> bool;
    // into inner??
}

pub struct QualificationMatchAuto {
    red: AllianceInfo<TraditionalJunction, 2>,
    blue: AllianceInfo<TraditionalJunction, 2>,
    // includes cones and beacons
    possessions: HashMap<TraditionalJunction, Alliance>
}

#[repr(transparent)]
pub struct QualificationMatchTeleOp(QualificationMatchAuto);

#[repr(transparent)]
pub struct QualificationMatchEndGame(QualificationMatchAuto);

impl QualificationMatchAuto {
    fn has_beacon_on(&self, location: TraditionalJunction) -> bool {
        self.red.beacon_placements.contains(&Valid(location)) ||
            self.blue.beacon_placements.contains(&Valid(location))
    }

    fn new(red: [FtcTeamID; 2], blue: [FtcTeamID; 2], unchecked: bool) -> Self {
        if unchecked || Self::verify_teams(red, blue) {
            Self {
                red: AllianceInfo::new(red),
                blue: AllianceInfo::new(blue),
                possessions: HashMap::new()
            }
        } else {
            panic!("The same team cannot compete in two slots in the same match.")
        }
    }

    fn verify_teams(red: [FtcTeamID; 2], blue: [FtcTeamID; 2]) -> bool {
        let [r1, r2] = red;
        let [b1, b2] = blue;
        r1 != r2 && r1 != b1 && r1 != b2
            && r2 != b1 && r2 != b2 && b1 != b2
    }
}

pub enum BeaconError {
    JunctionIsCapped, BeaconPreviouslyScored, RobotNotInMatch
}

impl Match<TraditionalJunction, 2> for QualificationMatchAuto {
    // returns true if modification was successful
    fn add_cone(&mut self, alliance: Alliance, location: TraditionalJunction) -> bool {
        let alliance_info = &mut self[alliance];
        if self.has_beacon_on(location) { return false; }
        let amounts = &mut alliance_info.cone_amounts;
        amounts.insert(
            location,
            unsafe {
                NonZeroU8::new_unchecked(
                    transmute::<Option<NonZeroU8>, u8>(amounts.get(&location).map(|x| *x)).saturating_add(1)
                )
            }
        );
        self.possessions.insert(location, alliance);
        true
    }
    fn add_terminal(&mut self, alliance: Alliance, terminal: Terminal) -> bool {
        let amounts = &mut self[alliance].terminal_amounts;
        let near_terminal = terminal == Terminal::Near;
        if near_terminal {
            amounts.0 += 1;
        } else {
            amounts.1 += 1;
        }
        near_terminal
    }

    type BeaconErrorType = BeaconError;
    fn add_beacon(&mut self, robot: FtcTeamID, location: TraditionalJunction) -> Result<(), Self::BeaconErrorType> {
        let alliance = self.alliance_of(robot);
        if alliance == None { return Err(RobotNotInMatch); }
        let alliance = alliance.unwrap();
        let alliance_info = &mut self[alliance];
        let team_index = alliance_info.teams.iter().position(|t| *t == robot).unwrap();
        if alliance_info.beacon_placements[team_index] != MaybeInvalidJunction::None { return Err(BeaconPreviouslyScored); }
        if self.has_beacon_on(location) { return Err(JunctionIsCapped); }
        alliance_info.beacon_placements[team_index] = Valid(location);
        self.possessions.insert(location, alliance);
        Ok(())
    }

    fn penalty(&mut self, alliance: Alliance, points: u8) {
        self[alliance].penalty_points += points as u16;
    }

    fn alliance_of(&self, robot: FtcTeamID) -> Option<Alliance> {
        for alliance in [Alliance::RED, Alliance::BLUE] {
            if self[alliance].teams.contains(&robot) { return Some(alliance); }
        }
        None
    }
}

impl Auto<TraditionalJunction, 2> for QualificationMatchAuto {
    type TeleOpType = QualificationMatchTeleOp;

    fn new(red: [FtcTeamID; 2], blue: [FtcTeamID; 2]) -> Self {
        Self::new(red, blue, false)
    }

    fn try_new(red: [FtcTeamID; 2], blue: [FtcTeamID; 2]) -> Option<Self> {
        if Self::verify_teams(red, blue) {
            Some(Self::new(red, blue, true))
        } else {
            None
        }
    }

    fn park(&mut self, robot: FtcTeamID, location: ParkingLocation) -> bool {
        let alliance = self.alliance_of(robot);
        if alliance == None { return false; }
        let alliance = alliance.unwrap();
        let alliance_info = &mut self[alliance];
        let team_index = alliance_info.teams.iter().position(|t| *t == robot).unwrap();
        alliance_info.parking_locations[team_index] = Some(location);
        true
    }

    fn into_teleop(mut self) -> Self::TeleOpType {
        for alliance_info in [&mut self.red, &mut self.blue] {
            alliance_info.auto_points =
                alliance_info.cone_amounts.values().sum() + alliance_info.terminal_amounts.0
        }
        unsafe { transmute(self) }
    }
}

macro_rules! delegated_impl {
    ($trai:ty, $struc:ty) => {
        impl $trai for $struc {
            #[inline(always)]
            fn add_cone(&mut self, alliance: Alliance, location: TraditionalJunction) -> bool {
                self.0.add_cone(alliance, location)
            }

            #[inline(always)]
            fn add_terminal(&mut self, alliance: Alliance, terminal: Terminal) -> bool {
                self.0.add_terminal(alliance, terminal);
                true
            }

            type BeaconErrorType = BeaconError;
            #[inline(always)]
            fn add_beacon(&mut self, robot: FtcTeamID, location: TraditionalJunction) -> Result<(), Self::BeaconErrorType> {
                self.0.add_beacon(robot, location)
            }

            #[inline(always)]
            fn penalty(&mut self, alliance: Alliance, points: u8) {
                self.0.penalty(alliance, points)
            }

            #[inline(always)]
            fn alliance_of(&self, robot: FtcTeamID) -> Option<Alliance> {
                self.0.alliance_of(robot)
            }
        }
    }
}

delegated_impl!(Match<TraditionalJunction, 2>, QualificationMatchTeleOp);
delegated_impl!(Match<TraditionalJunction, 2>, QualificationMatchEndGame);

impl TeleOp<TraditionalJunction, 2> for QualificationMatchTeleOp {
    type EndGameType = QualificationMatchEndGame;

    #[inline(always)]
    fn into_end_game(self) -> Self::EndGameType {
        unsafe { transmute(self) }
    }
}

impl EndGame<TraditionalJunction, 2> for QualificationMatchEndGame {
    fn park_in_terminal(robot: FtcTeamID) -> bool {
        todo!()
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Alliance {
    RED, BLUE
}

impl Index<Alliance> for QualificationMatchAuto {
    type Output = AllianceInfo<TraditionalJunction, 2>;

    fn index(&self, index: Alliance) -> &Self::Output {
        match index {
            Alliance::RED => &self.red,
            Alliance::BLUE => &self.blue
        }
    }
}

impl IndexMut<Alliance> for QualificationMatchAuto {
    fn index_mut(&mut self, index: Alliance) -> &mut Self::Output {
        match index {
            Alliance::RED => &mut self.red,
            Alliance::BLUE => &mut self.blue
        }
    }
}

// possession is handled by the Match implementation
#[derive(Debug)]
struct AllianceInfo<T: Junction, const N: usize> {
    teams: [FtcTeamID; N],
    penalty_points: u16,
    auto_points: u16, // aka TBP1
    cone_amounts: HashMap<T, NonZeroU8>, // how many cones are on each junction?
    terminal_amounts: (u8, u8),
    beacon_placements: [MaybeInvalidJunction<T>; N],
    parking_locations: [Option<ParkingLocation>; N]
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, Hash)]
enum MaybeInvalidJunction<T: Junction> {
    Valid(T), Invalid, None
}

impl <T: Junction, const N: usize> AllianceInfo<T, N> {
    fn new(teams: [FtcTeamID; N]) -> Self {
        Self {
            teams,
            penalty_points: 0,
            auto_points: 0,
            cone_amounts: HashMap::new(),
            terminal_amounts: (0, 0),
            beacon_placements: [MaybeInvalidJunction::None; N],
            parking_locations: [None; N]
        }
    }
}

// probably requires an rc of cell, or unsafe fuckery
// pub struct Robot<'a, T: Junction, const N: usize, M: Match<T, N>> {
//     robot_match: &'a UnsafeCell<M>,
//     id: FtcTeamID,
//     _no_send: PhantomData<*mut ()>
// }

