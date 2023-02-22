use std::collections::HashMap;
use std::hint::unreachable_unchecked;
use std::num::NonZeroU8;
use std::ops::{Index, IndexMut};
use std::mem::{MaybeUninit, transmute};
use BeaconError::*;
use MaybeInvalidJunction::{Invalid, Valid};
use crate::traditional::TraditionalJunction;

#[cfg(test)]
mod tests;

mod traditional;

// TODO decide what is public - index traits + alliance info?
// TODO index and alliance or FTC ID?
// TODO descoring?

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
    fn new(red: [(FtcTeamID, bool); N], blue: [(FtcTeamID, bool); N], signal_zone: SignalZone) -> Self
    where Self: Sized {
        let option = Self::try_new(red, blue, signal_zone);
        if let Some(robot_match) = option {
            robot_match
        } else {
            panic!("A problem occurred while trying to construct a match.")
        }
    }
    fn try_new(red: [(FtcTeamID, bool); N], blue: [(FtcTeamID, bool); N], signal_zone: SignalZone) -> Option<Self>
    where Self: Sized;
    fn park(&mut self, robot: FtcTeamID, location: ParkingLocation) -> bool;
    fn into_teleop(self) -> Self::TeleOpType;
}
pub trait TeleOp<T: Junction, const N: usize>: Match<T, N> {
    type EndGameType: EndGame<T, N>;
    fn into_end_game(self) -> Self::EndGameType;
}
pub trait EndGame<T: Junction, const N: usize>: Match<T, N> {
    fn park_in_terminal(&mut self, robot: FtcTeamID) -> bool;
    fn end_match(self) -> [AllianceInfo<N>; 2];
}

struct InternalQualificationMatchAuto {
    red: InternalAllianceInfo<TraditionalJunction, 2>,
    blue: InternalAllianceInfo<TraditionalJunction, 2>,
    // includes cones and beacons
    possessions: HashMap<TraditionalJunction, Alliance>
}

pub struct QualificationMatchAuto {
    data: InternalQualificationMatchAuto,
    red_signal_sleeves: [bool; 2],
    blue_signal_sleeves: [bool; 2],
    signal_zone: SignalZone
}

#[repr(transparent)]
pub struct QualificationMatchTeleOp(InternalQualificationMatchAuto);

#[repr(transparent)]
pub struct QualificationMatchEndGame(InternalQualificationMatchAuto);

impl InternalQualificationMatchAuto {
    fn has_beacon_on(&self, location: TraditionalJunction) -> bool {
        self.red.beacon_placements.contains(&Valid(location)) ||
            self.blue.beacon_placements.contains(&Valid(location))
    }

    fn new(red: [FtcTeamID; 2], blue: [FtcTeamID; 2], unchecked: bool) -> Self {
        if unchecked || Self::verify_teams(red, blue) {
            Self {
                red: InternalAllianceInfo::new(red),
                blue: InternalAllianceInfo::new(blue),
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

    fn park(&mut self, robot: FtcTeamID, location: ParkingLocation) -> bool {
        let alliance = self.alliance_of(robot);
        if alliance == None { return false; }
        let alliance = alliance.unwrap();
        let alliance_info = &mut self[alliance];
        let team_index = alliance_info.teams.iter().position(|t| *t == robot).unwrap();
        alliance_info.parking_locations[team_index] = Some(location);
        true
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, Hash)]
#[repr(u8)]
pub enum BeaconError {
    JunctionIsCapped, BeaconPreviouslyScored, RobotNotInMatch, ScoredOutsideEndgame
}

impl Match<TraditionalJunction, 2> for InternalQualificationMatchAuto {
    // returns true if modification was successful
    fn add_cone(&mut self, alliance: Alliance, location: TraditionalJunction) -> bool {
        if self.has_beacon_on(location) { return false; }
        let alliance_info = &mut self[alliance];
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
        let mut output = Ok(());
        if self.has_beacon_on(location) {
            output = Err(JunctionIsCapped);
        }
        let alliance = alliance.unwrap();
        let alliance_info = &mut self[alliance];
        let team_index = alliance_info.teams.iter().position(|t| *t == robot).unwrap();
        if alliance_info.beacon_placements[team_index] != MaybeInvalidJunction::None { return Err(BeaconPreviouslyScored); }
        match output {
            Ok(()) => {
                alliance_info.beacon_placements[team_index] = Valid(location);
                self.possessions.insert(location, alliance);
            },
            Err(JunctionIsCapped) => alliance_info.beacon_placements[team_index] = Invalid,
            _ => unsafe { unreachable_unchecked() }
        };
        output
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

impl QualificationMatchAuto {
    fn alliance_data(&mut self) -> ([(&mut InternalAllianceInfo<TraditionalJunction, 2>, [bool; 2]); 2], SignalZone) {
        ([(&mut self.data.red, self.red_signal_sleeves), (&mut self.data.blue, self.blue_signal_sleeves)], self.signal_zone)
    }
}
impl Auto<TraditionalJunction, 2> for QualificationMatchAuto {
    type TeleOpType = QualificationMatchTeleOp;

    fn new(red: [(FtcTeamID, bool); 2], blue: [(FtcTeamID, bool); 2], signal_zone: SignalZone) -> Self {
        let [(red1, red1sleeve), (red2, red2sleeve)] = red;
        let [(blue1, blue1sleeve), (blue2, blue2sleeve)] = blue;
        Self {
            data: InternalQualificationMatchAuto::new([red1, red2], [blue1, blue2], false),
            red_signal_sleeves: [red1sleeve, red2sleeve],
            blue_signal_sleeves: [blue1sleeve, blue2sleeve],
            signal_zone
        }
    }

    fn try_new(red: [(FtcTeamID, bool); 2], blue: [(FtcTeamID, bool); 2], signal_zone: SignalZone) -> Option<Self> {
        let [(red1, red1sleeve), (red2, red2sleeve)] = red;
        let [(blue1, blue1sleeve), (blue2, blue2sleeve)] = blue;
        let reds = [red1, red2];
        let blues = [blue1, blue2];
        if InternalQualificationMatchAuto::verify_teams(reds, blues) {
            Some(
                Self {
                    data: InternalQualificationMatchAuto::new(reds, blues, true),
                    red_signal_sleeves: [red1sleeve, red2sleeve],
                    blue_signal_sleeves: [blue1sleeve, blue2sleeve],
                    signal_zone
                }
            )
        } else {
            None
        }
    }

    #[inline(always)]
    fn park(&mut self, robot: FtcTeamID, location: ParkingLocation) -> bool {
        self.data.park(robot, location)
    }

    fn into_teleop(mut self) -> Self::TeleOpType {
        let (alliance_data, signal_zone) = self.alliance_data();
        for (alliance_info, signal_sleeves) in alliance_data {
            alliance_info.auto_points =
                (alliance_info.cone_amounts.iter().map(|(junction, count)| count.get() * junction.points()).sum::<u8>() +
                    alliance_info.terminal_amounts.0 +
                    (0..2).map(|i|
                        match alliance_info.parking_locations[i] {
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
                ) as u16;
            alliance_info.parking_locations = [None; 2];
        }
        unsafe { transmute(self.data) }
    }
}

macro_rules! delegated_impl {
    ($struc:ty, $delegate:tt, $beacon_impl:item $(, $result:literal)?) => {
        impl Match<TraditionalJunction, 2> for $struc {
            #[inline(always)]
            fn add_cone(&mut self, alliance: Alliance, location: TraditionalJunction) -> bool {
                self.$delegate.add_cone(alliance, location)
            }

            #[inline(always)]
            fn add_terminal(&mut self, alliance: Alliance, terminal: Terminal) -> bool {
                self.$delegate.add_terminal(alliance, terminal)
                $( ; $result )?
            }

            type BeaconErrorType = BeaconError;
            $beacon_impl

            #[inline(always)]
            fn penalty(&mut self, alliance: Alliance, points: u8) {
                self.$delegate.penalty(alliance, points)
            }

            #[inline(always)]
            fn alliance_of(&self, robot: FtcTeamID) -> Option<Alliance> {
                self.$delegate.alliance_of(robot)
            }
        }
    };
}

macro_rules! beacon {
    ($delegate:tt) => {
        fn add_beacon(&mut self, robot: FtcTeamID, _: TraditionalJunction) -> Result<(), BeaconError> {
            let alliance = self.alliance_of(robot);
            if alliance == None { return Err(RobotNotInMatch); }
            let alliance = alliance.unwrap();
            let alliance_info = &mut self.$delegate[alliance];
            let team_index = alliance_info.teams.iter().position(|t| *t == robot).unwrap();
            alliance_info.beacon_placements[team_index] = Invalid;
            Err(ScoredOutsideEndgame)
        }
    };
}

delegated_impl!(QualificationMatchAuto, data, beacon! { data });
delegated_impl!(QualificationMatchTeleOp, 0, beacon! { 0 }, true);
delegated_impl!(QualificationMatchEndGame, 0,
    fn add_beacon(&mut self, robot: FtcTeamID, location: TraditionalJunction) -> Result<(), BeaconError> {
        #![inline(always)]
        self.0.add_beacon(robot, location)
    }, true
);

impl TeleOp<TraditionalJunction, 2> for QualificationMatchTeleOp {
    type EndGameType = QualificationMatchEndGame;

    #[inline(always)]
    fn into_end_game(self) -> Self::EndGameType {
        unsafe { transmute(self) }
    }
}

impl EndGame<TraditionalJunction, 2> for QualificationMatchEndGame {
    fn park_in_terminal(&mut self, robot: FtcTeamID) -> bool {
        // exact terminal location does not matter
        self.0.park(robot, ParkingLocation::NearTerminal)
    }

    fn end_match(self) -> [AllianceInfo<2>; 2] {
        // let mut output: [MaybeUninit<AllianceInfo<2>>; 2] = [MaybeUninit::uninit(); 2];
        // let alliances = [Alliance::RED, Alliance::BLUE];
        // for i in 0..2 {
        //     let alliance = alliances[i];
        //     let alliance_info = &self.0[alliance];
        //     output[i] = MaybeUninit::new(
        //         AllianceInfo {
        //             alliance,
        //             auto_points: alliance_info.auto_points,
        //
        //         }
        //     )
        // }
        todo!()
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Alliance {
    RED, BLUE
}

impl Index<Alliance> for InternalQualificationMatchAuto {
    type Output = InternalAllianceInfo<TraditionalJunction, 2>;

    fn index(&self, index: Alliance) -> &Self::Output {
        match index {
            Alliance::RED => &self.red,
            Alliance::BLUE => &self.blue
        }
    }
}

impl IndexMut<Alliance> for InternalQualificationMatchAuto {
    fn index_mut(&mut self, index: Alliance) -> &mut Self::Output {
        match index {
            Alliance::RED => &mut self.red,
            Alliance::BLUE => &mut self.blue
        }
    }
}

// possession is handled by the Match implementation
#[derive(Debug)]
struct InternalAllianceInfo<T: Junction, const N: usize> {
    teams: [FtcTeamID; N],
    penalty_points: u16,
    auto_points: u16, // aka TBP1
    cone_amounts: HashMap<T, NonZeroU8>, // how many cones are on each junction?
    terminal_amounts: (u8, u8),
    beacon_placements: [MaybeInvalidJunction<T>; N],
    parking_locations: [Option<ParkingLocation>; N]
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

impl <T: Junction, const N: usize> InternalAllianceInfo<T, N> {
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

