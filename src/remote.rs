use crate::id::{Alliance, FtcTeamID, MatchIndex};
use crate::locations::{ParkingLocation, SignalZone, Terminal};
use crate::sealed::Sealed;
use crate::BeaconError::{BeaconPreviouslyScored, JunctionIsCapped};
use crate::ConeRemovalError::{BeaconOnJunction, JunctionIsEmpty};
use crate::MaybeInvalidJunction::{Invalid, Valid};
use crate::{AllianceInfo, InternalAllianceInfo};
use crate::{Auto, BeaconError, ConeRemovalError, FieldCoordinate, Match, MaybeInvalidJunction};
use crate::{BeaconScoredOutsideEndgame, EndGame, TeleOp};
use nohash::IntMap;
use std::fmt::{Display, Formatter};
use std::mem::transmute;
use std::num::NonZeroU8;
use std::ops::Index;
use RedRemoteJunction::*;

#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug, Hash)]
#[repr(u8)]
pub enum RemoteCircuitPattern {
    Pattern1 = 0,
    Pattern2,
    Pattern3,
    Pattern4,
    Pattern5,
    Pattern6,
}

// TODO this is very similar to a potential Robot API. hmm.
pub trait RemoteMatch<T: FieldCoordinate>: Match<T> {
    fn add_cone(&mut self, location: T) -> bool;
    fn add_terminal(&mut self, terminal: Terminal) -> bool;
    fn add_beacon(&mut self, location: T) -> Result<(), Self::BeaconErrorType>;
    fn penalty(&mut self, points: u8);
    fn team_id(&self) -> FtcTeamID;
    // TODO do we have to copy Match methods to avoid needing UFCS
}
pub trait RemoteAuto<T: FieldCoordinate, const R: usize, const B: usize>
where
    Self: RemoteMatch<T> + Auto<T, R, B>,
    Self::TeleOpType: RemoteTeleOp<T, R, B>,
    <Self::TeleOpType as TeleOp<T, R, B>>::EndGameType: RemoteEndGame<T, R, B>,
{
    fn with_team(
        team: FtcTeamID,
        has_signal_sleeve: bool,
        signal_zone: SignalZone,
        circuit_pattern: RemoteCircuitPattern,
    ) -> Self
    where Self: Sized;
    #[inline(always)]
    /// Creates a remote match with the single participant having an ID of -1.
    fn new(
        has_signal_sleeve: bool,
        signal_zone: SignalZone,
        circuit_pattern: RemoteCircuitPattern,
    ) -> Self
    where Self: Sized {
        Self::with_team(FtcTeamID(-1), has_signal_sleeve, signal_zone, circuit_pattern)
    }
    fn park(&mut self, location: impl Into<ParkingLocation>);
}

pub trait RemoteTeleOp<T: FieldCoordinate, const R: usize, const B: usize>
where
    Self: RemoteMatch<T> + TeleOp<T, R, B>,
    Self::EndGameType: RemoteEndGame<T, R, B>,
{
}

pub trait RemoteEndGame<T: FieldCoordinate, const R: usize, const B: usize>:
    RemoteMatch<T> + EndGame<T, R, B>
{
    fn park_in_terminal(&mut self);
    fn end_match(self) -> AllianceInfo<1>;
}

const CIRCUIT_PATTERNS: [&[RedRemoteJunction]; 6] = [
    &[Z2, Z3, Z4, Z5, Y5],
    &[X1, Y1, Z1, X2, Y3, X4, Y5],
    &[Z1, Y2, Z3, Y4, X5],
    &[Z1, Y2, Y3, Y4, X5],
    &[Y1, Z1, X2, Y3, X4, X5],
    &[X1, X2, X3, X4, X5, Y1, Y2, Y3, Y4, Y5, Z1, Z2, Z3, Z4, Z5],
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
    junctions: IntMap<RedRemoteJunction, NonZeroU8>,
}

impl InternalRemoteMatch {
    fn new(team: FtcTeamID, circuit_pattern: RemoteCircuitPattern) -> Self {
        Self {
            data: InternalAllianceInfo::new([team]),
            circuit_pattern,
            junctions: IntMap::default(),
        }
    }
}

impl InternalRemoteMatch {
    #[inline]
    fn has_beacon_on(&self, location: RedRemoteJunction) -> bool {
        self.data.beacon_placements.contains(&Valid(location))
    }

    fn add_cone(&mut self, location: RedRemoteJunction) -> bool {
        if self.data.beacon_placements[0] != Valid(location) {
            match self.junctions.get_mut(&location) {
                Some(num) => match (*num).checked_add(1) {
                    Some(value) => {
                        self.junctions.insert(location, value);
                    }
                    None => panic!("The cone stack at {location} has overflown."),
                },
                None => {
                    self.junctions
                        .insert(location, unsafe { NonZeroU8::new_unchecked(1) });
                }
            }
            true
        } else {
            false
        }
    }

    fn remove_cone(
        &mut self,
        alliance: Alliance,
        location: RedRemoteJunction,
    ) -> Result<Alliance, ConeRemovalError> {
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
                    Ok(alliance)
                }
                None => Err(JunctionIsEmpty),
            }
        } else {
            Err(BeaconOnJunction)
        }
    }

    fn add_terminal(&mut self, terminal: Terminal) -> bool {
        let amounts = &mut self.data.terminal_amounts;
        let near_terminal = terminal == Terminal::Near;
        amounts[near_terminal as usize] += 1;
        near_terminal
    }

    fn add_beacon(&mut self, location: RedRemoteJunction) -> Result<(), BeaconError> {
        // Index verification is handled by the implementor. This has a hardcoded index of 0.
        if self.has_beacon_on(location) {
            Err(JunctionIsCapped)
        } else if self.data.beacon_placements[0] != MaybeInvalidJunction::None {
            Err(BeaconPreviouslyScored)
        } else {
            self.data.beacon_placements[0] = Valid(location);
            Ok(())
        }
    }

    #[inline(always)]
    fn penalty(&mut self, points: u8) {
        self.data.penalty_points += points as u16;
    }
}

// TODO seeing if optimizing these two fields into one, along with the circuit pattern field, is worth it
pub struct RedRemoteAuto {
    data: InternalRemoteMatch,
    has_signal_sleeve: bool,
    signal_zone: SignalZone,
}

#[repr(transparent)]
pub struct RedRemoteTeleOp(InternalRemoteMatch);
#[repr(transparent)]
pub struct RedRemoteEndGame(InternalRemoteMatch);

macro_rules! check_red_captain {
    ($index:expr) => {
        if $index != MatchIndex::RED_CAPTAIN {
            panic!(
                "Index {} invalid for a red remote match, which only has a single red player with index 0.",
                $index
            )
        }
    };
}

macro_rules! panic_if_blue {
    ($alliance:expr) => {
        if $alliance.is_blue() {
            panic!("Red remote matches do not have a blue alliance.")
        }
    };
}

macro_rules! check_blue_captain {
    ($index:expr) => {
        if $index != MatchIndex::BLUE_CAPTAIN {
            panic!(
                "Index {} invalid for a blue remote match, which only has a single blue player with index 0.",
                $index
            )
        }
    };
}

macro_rules! panic_if_red {
    ($alliance:expr) => {
        if !$alliance.is_blue() {
            panic!("Blue remote matches do not have a red alliance.")
        }
    };
}

macro_rules! delegated_impl {
    ($( |$d2:ident, )? $struc:ty, $delegate:tt, $alliance:ident, $captain:ident, $junction_type:ty, $beacon_err_type:ty, $alliance_checker:tt, $index_checker:tt, $( $result:literal, )? ($( $beacon_impl:tt )+)) => {
        impl Sealed for $struc {}
        impl Index<Alliance> for $struc {
            type Output = [FtcTeamID];

            fn index(&self, index: Alliance) -> &Self::Output {
                match index {
                    Alliance::$alliance => &self.$( $d2. )?$delegate.data.teams,
                    _ => &[]
                }
            }
        }

        impl Index<MatchIndex> for $struc {
            type Output = FtcTeamID;

            fn index(&self, index: MatchIndex) -> &Self::Output {
                $index_checker!(index);
                &self.$( $d2. )?$delegate.data.teams[0]
            }
        }

        impl Match<$junction_type> for $struc {
            #[inline]
            fn add_cone(&mut self, alliance: Alliance, location: $junction_type) -> bool {
                $alliance_checker!(alliance);
                <Self as RemoteMatch<$junction_type>>::add_cone(self, location)
            }

            type ConeRemovalErrorType = ConeRemovalError;
            #[inline(always)]
            fn remove_cone(&mut self, location: $junction_type) -> Result<Alliance, Self::ConeRemovalErrorType> {
                self.$( $d2. )?$delegate.remove_cone(Alliance::$alliance, unsafe { transmute(location) })
            }

            #[inline]
            fn add_terminal(&mut self, alliance: Alliance, terminal: Terminal) -> bool {
                $alliance_checker!(alliance);
                <Self as RemoteMatch<$junction_type>>::add_terminal(self, terminal)
            }

            type BeaconErrorType = $beacon_err_type;
            #[inline]
            fn add_beacon(&mut self, robot: MatchIndex, location: $junction_type) -> Result<(), Self::BeaconErrorType> {
                $index_checker!(robot);
                <Self as RemoteMatch<$junction_type>>::add_beacon(self, location)
            }

            #[inline]
            fn penalty(&mut self, alliance: Alliance, points: u8) {
                $alliance_checker!(alliance);
                <Self as RemoteMatch<$junction_type>>::penalty(self, points)
            }

            fn alliance_of(&self, robot: FtcTeamID) -> Option<Alliance> {
                if self.$( $d2. )?$delegate.data.teams[0] == robot {
                    Some(Alliance::$alliance)
                } else {
                    None
                }
            }

            fn index_of(&self, robot: FtcTeamID) -> Option<MatchIndex> {
                if self.$( $d2. )?$delegate.data.teams[0] == robot {
                    Some(MatchIndex::$captain)
                } else {
                    None
                }
            }
        }

        impl RemoteMatch<$junction_type> for $struc {
            #[inline(always)]
            fn add_cone(&mut self, location: $junction_type) -> bool {
                self.$( $d2. )?$delegate.add_cone(unsafe { transmute(location) })
            }

            #[inline(always)]
            fn add_terminal(&mut self, terminal: Terminal) -> bool {
                self.$( $d2. )?$delegate.add_terminal(terminal)
                $( ; $result )?
            }

            $( $beacon_impl )+

            #[inline(always)]
            fn penalty(&mut self, points: u8) {
                self.$( $d2. )?$delegate.penalty(points)
            }

            #[inline(always)]
            fn team_id(&self) -> FtcTeamID {
                self.$( $d2. )?$delegate.data.teams[0]
            }
        }
    };
}

// TODO copied method impls. do we care?
macro_rules! red_delegated_impl {
    ($struc:ty, $delegate:tt $( , $result:literal )?) => {
        red_delegated_impl!($struc, $delegate, BeaconScoredOutsideEndgame, $( $result, )? (
            #[inline]
            fn add_beacon(&mut self, _: RedRemoteJunction) -> Result<(), Self::BeaconErrorType> {
                self.$delegate.data.beacon_placements[0] = Invalid;
                Err(BeaconScoredOutsideEndgame)
            }
        ));
    };
    ($struc:ty, $delegate:tt, $beacon_err_type:ty, $( $result:literal, )? ($( $beacon_impl:tt )+)) => {
        delegated_impl! {
            $struc,
            $delegate,
            RED,
            RED_CAPTAIN,
            RedRemoteJunction,
            $beacon_err_type,
            panic_if_blue,
            check_red_captain,
            $( $result, )?
            ($( $beacon_impl )+)
        }
    };
}

// TODO for now i will panic if the wrong alliance is used, but we may want a better solution.
red_delegated_impl!(RedRemoteAuto, data);
red_delegated_impl!(RedRemoteTeleOp, 0);
red_delegated_impl!(RedRemoteEndGame, 0, BeaconError, (
    #[inline(always)]
    fn add_beacon(&mut self, location: RedRemoteJunction) -> Result<(), Self::BeaconErrorType> {
        self.0.add_beacon(location)
    }
));

impl Auto<RedRemoteJunction, 1, 0> for RedRemoteAuto {
    type TeleOpType = RedRemoteTeleOp;

    #[inline]
    fn park(&mut self, robot: MatchIndex, location: impl Into<ParkingLocation>) {
        check_red_captain!(robot);
        <Self as RemoteAuto<RedRemoteJunction, 1, 0>>::park(self, location)
    }

    fn into_teleop(mut self) -> Self::TeleOpType {
        self.data.data.auto_points = self
            .data
            .junctions
            .iter()
            .map(|(junction, &count)| (junction.points() * count.get()) as u16)
            .sum();
        self.data
            .data
            .score_auto_parking_terminals([self.has_signal_sleeve], self.signal_zone);
        unsafe { transmute(self.data) }
    }
}

impl RemoteAuto<RedRemoteJunction, 1, 0> for RedRemoteAuto {
    #[inline]
    fn with_team(
        team: FtcTeamID,
        has_signal_sleeve: bool,
        signal_zone: SignalZone,
        circuit_pattern: RemoteCircuitPattern,
    ) -> Self {
        Self {
            data: InternalRemoteMatch::new(team, circuit_pattern),
            has_signal_sleeve,
            signal_zone,
        }
    }

    #[inline(always)]
    fn park(&mut self, location: impl Into<ParkingLocation>) {
        self.data.data.parking_locations[0] = Some(location.into())
    }
}

impl TeleOp<RedRemoteJunction, 1, 0> for RedRemoteTeleOp {
    type EndGameType = RedRemoteEndGame;

    fn into_end_game(self) -> Self::EndGameType {
        unsafe { transmute(self) }
    }
}

impl RemoteTeleOp<RedRemoteJunction, 1, 0> for RedRemoteTeleOp {}

impl EndGame<RedRemoteJunction, 1, 0> for RedRemoteEndGame {
    #[inline]
    fn park_in_terminal(&mut self, robot: MatchIndex) {
        check_red_captain!(robot);
        <Self as RemoteEndGame<RedRemoteJunction, 1, 0>>::park_in_terminal(self)
    }

    fn end_match(self) -> (AllianceInfo<1>, AllianceInfo<0>) {
        (
            <Self as RemoteEndGame<RedRemoteJunction, 1, 0>>::end_match(self),
            AllianceInfo {
                alliance: Alliance::BLUE,
                teams: [],
                penalty_points: 0,
                auto_points: 0,
                teleop_points: 0,
                endgame_points: 0,
            }
        )
    }
}

impl RemoteEndGame<RedRemoteJunction, 1, 0> for RedRemoteEndGame {
    #[inline]
    fn park_in_terminal(&mut self) {
        // exact terminal location does not matter
        self.0.data.terminal_amounts[0] += 1;
    }

    fn end_match(mut self) -> AllianceInfo<1> {
        AllianceInfo {
            alliance: Alliance::RED,
            teams: self.0.data.teams,
            penalty_points: self.0.data.penalty_points,
            auto_points: self.0.data.auto_points,
            teleop_points: self.0.junctions.iter()
                .map(|(junction, count)| (
                    count.get() * junction.points()) as u16
                ).sum(),
            endgame_points: {
                let mut points = 0;
                // beacons
                let beacons = self.0.data.beacon_placements;
                let mut valid_beacon_count: u16 = 0;
                for beacon in beacons {
                    if let Valid(junction) = beacon {
                        // this inserts garbage that you should never read. we already calculated scored cones. TODO hmmmmm insertion
                        self.0.junctions.insert(junction, unsafe { NonZeroU8::new_unchecked(255) });
                        valid_beacon_count += 1;
                        points += 10;
                    }
                }
                // circuit logic
                points += {
                    let circuit_pattern = CIRCUIT_PATTERNS[self.0.circuit_pattern as usize];
                    if self.0.junctions.len() != circuit_pattern.len()
                        || self.0.data.terminal_amounts[0] == 0
                        || self.0.data.terminal_amounts[1] == 0{
                        0
                    } else {
                        circuit_pattern.iter().all(|key| self.0.junctions.contains_key(key)) as u16 * 20
                    }
                };
                // possessions
                points += (self.0.junctions.len() as u16 - valid_beacon_count) * 3;
                points
            },
        }
    }
}

#[repr(transparent)]
pub struct BlueRemoteAuto {
    inner: RedRemoteAuto
}
#[repr(transparent)]
pub struct BlueRemoteTeleOp {
    inner: RedRemoteTeleOp
}
#[repr(transparent)]
pub struct BlueRemoteEndGame {
    inner: RedRemoteEndGame
}

macro_rules! blue_delegated_impl {
    ($struc:ty, $delegate:tt $( , $result:literal )?) => {
        blue_delegated_impl!($struc, $delegate, BeaconScoredOutsideEndgame, $( $result, )? (
            #[inline]
            fn add_beacon(&mut self, _: BlueRemoteJunction) -> Result<(), Self::BeaconErrorType> {
                self.inner.$delegate.data.beacon_placements[0] = Invalid;
                Err(BeaconScoredOutsideEndgame)
            }
        ));
    };
    ($struc:ty, $delegate:tt, $beacon_err_type:ty, $( $result:literal, )? ($( $beacon_impl:tt )+)) => {
        delegated_impl! {
            |inner,
            $struc,
            $delegate,
            BLUE,
            BLUE_CAPTAIN,
            BlueRemoteJunction,
            $beacon_err_type,
            panic_if_red,
            check_blue_captain,
            $( $result, )?
            ($( $beacon_impl )+)
        }
    };
}

blue_delegated_impl!(BlueRemoteAuto, data);
blue_delegated_impl!(BlueRemoteTeleOp, 0);
blue_delegated_impl!(BlueRemoteEndGame, 0, BeaconError, (
    #[inline(always)]
    fn add_beacon(&mut self, location: BlueRemoteJunction) -> Result<(), Self::BeaconErrorType> {
        <RedRemoteEndGame as RemoteMatch<RedRemoteJunction>>::add_beacon(&mut self.inner, unsafe { transmute(location) })
    }
));

impl Auto<BlueRemoteJunction, 0, 1> for BlueRemoteAuto {
    type TeleOpType = BlueRemoteTeleOp;

    #[inline]
    fn park(&mut self, robot: MatchIndex, location: impl Into<ParkingLocation>) {
        check_blue_captain!(robot);
        <Self as RemoteAuto<BlueRemoteJunction, 0, 1>>::park(self, location)
    }

    #[inline(always)]
    fn into_teleop(self) -> Self::TeleOpType {
        unsafe { transmute(self.inner.into_teleop()) }
    }
}

impl RemoteAuto<BlueRemoteJunction, 0, 1> for BlueRemoteAuto {
    #[inline(always)]
    fn with_team(team: FtcTeamID, has_signal_sleeve: bool, signal_zone: SignalZone, circuit_pattern: RemoteCircuitPattern) -> Self where Self: Sized {
        unsafe {
            transmute(RedRemoteAuto::with_team(team, has_signal_sleeve, signal_zone, circuit_pattern))
        }
    }

    fn park(&mut self, location: impl Into<ParkingLocation>) {
        <RedRemoteAuto as RemoteAuto<RedRemoteJunction, 1, 0>>::park(&mut self.inner, location)
    }
}

impl TeleOp<BlueRemoteJunction, 0, 1> for BlueRemoteTeleOp {
    type EndGameType = BlueRemoteEndGame;

    fn into_end_game(self) -> Self::EndGameType {
        unsafe { transmute(self) }
    }
}

impl RemoteTeleOp<BlueRemoteJunction, 0, 1> for BlueRemoteTeleOp {}

impl EndGame<BlueRemoteJunction, 0, 1> for BlueRemoteEndGame {
    #[inline]
    fn park_in_terminal(&mut self, robot: MatchIndex) {
        check_blue_captain!(robot);
        <Self as RemoteEndGame<BlueRemoteJunction, 0, 1>>::park_in_terminal(self)
    }

    fn end_match(self) -> (AllianceInfo<0>, AllianceInfo<1>) {
        (
            AllianceInfo {
                alliance: Alliance::RED,
                teams: [],
                penalty_points: 0,
                auto_points: 0,
                teleop_points: 0,
                endgame_points: 0,
            },
            <Self as RemoteEndGame<BlueRemoteJunction, 0, 1>>::end_match(self)
        )
    }
}

impl RemoteEndGame<BlueRemoteJunction, 0, 1> for BlueRemoteEndGame {
    #[inline(always)]
    fn park_in_terminal(&mut self) {
        <RedRemoteEndGame as RemoteEndGame<RedRemoteJunction, 1, 0>>::park_in_terminal(&mut self.inner)
    }

    fn end_match(self) -> AllianceInfo<1> {
        let mut info = <RedRemoteEndGame as RemoteEndGame<RedRemoteJunction, 1, 0>>::end_match(self.inner);
        info.alliance = Alliance::BLUE;
        info
    }
}
