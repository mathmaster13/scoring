use super::*;
use crate::sealed::Sealed;
use crate::BeaconError::*;
use crate::ConeRemovalError::{BeaconOnJunction, JunctionIsEmpty};
use crate::MaybeInvalid::{Invalid, Valid};
use nohash::IntMap;
use std::hint::unreachable_unchecked;
use std::mem::transmute;
use std::ops::Index;

#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug)]
#[repr(u8)]
// REPRESENTATION: [letter][number][junction points - 2]
// everything is zero-indexed
pub enum TraditionalJunction {
    V1 = 0b000_000_00,
    V2 = 0b000_001_01,
    V3 = 0b000_010_00,
    V4 = 0b000_011_01,
    V5 = 0b000_100_00,
    W1 = 0b001_000_01,
    W2 = 0b001_001_10,
    W3 = 0b001_010_11,
    W4 = 0b001_011_10,
    W5 = 0b001_100_01,
    X1 = 0b010_000_00,
    X2 = 0b010_001_11,
    X3 = 0b010_010_00,
    X4 = 0b010_011_11,
    X5 = 0b010_100_00,
    Y1 = 0b011_000_01,
    Y2 = 0b011_001_10,
    Y3 = 0b011_010_11,
    Y4 = 0b011_011_10,
    Y5 = 0b011_100_01,
    Z1 = 0b100_000_00,
    Z2 = 0b100_001_01,
    Z3 = 0b100_010_00,
    Z4 = 0b100_011_01,
    Z5 = 0b100_100_00,
}

junction_impl!(TraditionalJunction, 5, 5);

#[derive(Debug)]
struct InternalTraditionalMatch {
    red: InternalAllianceInfo<TraditionalJunction, 2>,
    blue: InternalAllianceInfo<TraditionalJunction, 2>,
    // beacons are not stored here!
    junctions: IntMap<TraditionalJunction, ConeStack>,
}

// like has_beacon_on, but inlined to appease the borrow checker
macro_rules! has_beacon_on {
    ($this:ident, $location:ident) => {
        $this.red.beacon_placements.contains(&Valid($location))
            || $this.blue.beacon_placements.contains(&Valid($location))
    };
}

impl InternalTraditionalMatch {
    fn data_of(&self, index: Alliance) -> &InternalAllianceInfo<TraditionalJunction, 2> {
        match index {
            Alliance::RED => &self.red,
            Alliance::BLUE => &self.blue,
        }
    }

    fn data_of_mut(
        &mut self,
        index: Alliance,
    ) -> &mut InternalAllianceInfo<TraditionalJunction, 2> {
        match index {
            Alliance::RED => &mut self.red,
            Alliance::BLUE => &mut self.blue,
        }
    }

    fn has_beacon_on(&self, location: TraditionalJunction) -> bool {
        has_beacon_on!(self, location)
    }

    fn new(red: [FtcTeamID; 2], blue: [FtcTeamID; 2], unchecked: bool) -> Self {
        if unchecked || Self::verify_teams(red, blue) {
            Self {
                red: InternalAllianceInfo::new(red),
                blue: InternalAllianceInfo::new(blue),
                junctions: IntMap::default(),
            }
        } else {
            panic!("The same team cannot compete in two slots in the same match.")
        }
    }

    fn verify_teams(red: [FtcTeamID; 2], blue: [FtcTeamID; 2]) -> bool {
        let [r1, r2] = red;
        let [b1, b2] = blue;
        r1 != r2 && r1 != b1 && r1 != b2 && r2 != b1 && r2 != b2 && b1 != b2
    }

    #[inline]
    fn park(&mut self, robot: MatchIndex, location: impl Into<ParkingLocation>) {
        self.data_of_mut(robot.alliance()).parking_locations[robot.index()] = Some(location.into());
    }
}

#[derive(Debug)]
pub struct TraditionalAuto {
    data: InternalTraditionalMatch,
    red_signal_sleeves: [bool; 2],
    blue_signal_sleeves: [bool; 2],
    signal_zone: SignalZone,
}

impl TraditionalAuto {
    /// Creates a new match with "dummy" teams with IDs of -1, -2, -3, and -4.
    pub fn new(
        red_signal_sleeves: [bool; 2],
        blue_signal_sleeves: [bool; 2],
        signal_zone: SignalZone,
    ) -> Self {
        Self {
            data: InternalTraditionalMatch::new(
                [FtcTeamID(-1), FtcTeamID(-2)],
                [FtcTeamID(-3), FtcTeamID(-4)],
                true,
            ),
            red_signal_sleeves,
            blue_signal_sleeves,
            signal_zone,
        }
    }
    /// Creates a new match with the given teams, panicking if a team occurs more than once in this match.
    pub fn from_teams(
        red: [(FtcTeamID, bool); 2],
        blue: [(FtcTeamID, bool); 2],
        signal_zone: SignalZone,
    ) -> Self {
        let [(red1, red1sleeve), (red2, red2sleeve)] = red;
        let [(blue1, blue1sleeve), (blue2, blue2sleeve)] = blue;
        Self {
            data: InternalTraditionalMatch::new([red1, red2], [blue1, blue2], false),
            red_signal_sleeves: [red1sleeve, red2sleeve],
            blue_signal_sleeves: [blue1sleeve, blue2sleeve],
            signal_zone,
        }
    }
    /// Creates a new match with the given teams, returning None if a team occurs more than once in this match.
    pub fn try_from_teams(
        red: [(FtcTeamID, bool); 2],
        blue: [(FtcTeamID, bool); 2],
        signal_zone: SignalZone,
    ) -> Option<Self> {
        let [(red1, red1sleeve), (red2, red2sleeve)] = red;
        let [(blue1, blue1sleeve), (blue2, blue2sleeve)] = blue;
        let reds = [red1, red2];
        let blues = [blue1, blue2];
        if InternalTraditionalMatch::verify_teams(reds, blues) {
            Some(Self {
                data: InternalTraditionalMatch::new(reds, blues, true),
                red_signal_sleeves: [red1sleeve, red2sleeve],
                blue_signal_sleeves: [blue1sleeve, blue2sleeve],
                signal_zone,
            })
        } else {
            None
        }
    }
}

#[repr(transparent)]
#[derive(Debug)]
pub struct TraditionalTeleOp(InternalTraditionalMatch);

#[repr(transparent)]
#[derive(Debug)]
pub struct TraditionalEndGame(InternalTraditionalMatch);

impl Index<MatchIndex> for InternalTraditionalMatch {
    type Output = FtcTeamID;

    fn index(&self, index: MatchIndex) -> &Self::Output {
        &self.data_of(index.alliance()).teams[index.index()]
    }
}

impl Index<Alliance> for InternalTraditionalMatch {
    type Output = [FtcTeamID];

    fn index(&self, index: Alliance) -> &Self::Output {
        &self.data_of(index).teams
    }
}

impl Sealed for InternalTraditionalMatch {}

impl Match<TraditionalJunction> for InternalTraditionalMatch {
    // returns true if modification was successful
    fn score_for(&mut self, alliance: Alliance, location: TraditionalJunction) -> bool {
        if self.has_beacon_on(location) {
            return false;
        }
        match self.junctions.get_mut(&location) {
            Some(cone_stack) => cone_stack.push(alliance),
            None => {
                self.junctions.insert(location, ConeStack::new(alliance));
            }
        }
        true
    }

    type ConeRemovalErrorType = ConeRemovalError;
    fn descore(
        &mut self,
        location: TraditionalJunction,
    ) -> Result<Alliance, Self::ConeRemovalErrorType> {
        match self.junctions.get_mut(&location) {
            Some(cone_stack) => {
                if has_beacon_on!(self, location) {
                    Err(BeaconOnJunction)
                } else {
                    let alliance = cone_stack.pop();
                    if cone_stack.is_empty() {
                        self.junctions.remove(&location);
                    }
                    Ok(alliance.expect("Empty cone stacks must be removed."))
                }
            }
            None => Err(JunctionIsEmpty),
        }
    }

    fn add_terminal_for(&mut self, alliance: Alliance, terminal: Terminal) -> bool {
        let amounts = &mut self.data_of_mut(alliance).terminal_amounts;
        let near_terminal = terminal == Terminal::Near;
        // if near_terminal {
        //     amounts.0 += 1;
        // } else {
        //     amounts.1 += 1;
        // }
        amounts[near_terminal as usize] += 1;
        near_terminal
    }

    type BeaconErrorType = BeaconError;
    fn cap_for(
        &mut self,
        robot: MatchIndex,
        location: TraditionalJunction,
    ) -> Result<(), Self::BeaconErrorType> {
        let output = if self.has_beacon_on(location) {
            Err(JunctionIsCapped)
        } else {
            Ok(())
        };
        let alliance_info = &mut self.data_of_mut(robot.alliance());
        let team_index = robot.index();
        // if micro optimizations are needed, try this since it does not compute team index unless it has to
        // let alliance = self.alliance_of(robot);
        // if alliance == None { return Err(BeaconError::RobotNotInMatch); }
        // let output = if self.has_beacon_on(location) {
        //     Err(JunctionIsCapped)
        // } else {
        //     Ok(())
        // };
        // let alliance = alliance.unwrap();
        // let alliance_info = &mut self.data_of_mut(alliance);
        // let team_index = unsafe {
        //     alliance_info.teams.iter().position(|t| *t == robot).unwrap_unchecked();
        // };

        if alliance_info.beacon_placements[team_index] != MaybeInvalid::None {
            return Err(BeaconPreviouslyScored);
        }
        alliance_info.beacon_placements[team_index] = match output {
            Ok(()) => Valid(location),
            Err(JunctionIsCapped) => Invalid,
            _ => unsafe { unreachable_unchecked() },
        };
        output
    }

    // fn descore_beacon(&mut self, location: TraditionalJunction) -> Option<MatchIndex> {
    //     // relies on the structure of MatchIndex
    //     let beacons = {
    //         let [r0, r1] = self.red.beacon_placements;
    //         let [b0, b1] = self.blue.beacon_placements;
    //         [r0, b0, r1, b1] // this order is super important
    //     };
    //     beacons.into_iter()
    //         .position(|loc| loc == Valid(location))
    //         .map(|idx| MatchIndex(idx as u8)) // basically a transmute without the compiler getting scared
    // }

    #[inline(always)]
    fn penalize(&mut self, alliance: Alliance, points: u8) {
        self.data_of_mut(alliance).penalty_points += points as u16;
    }

    fn alliance_of(&self, robot: FtcTeamID) -> Option<Alliance> {
        for alliance in [Alliance::RED, Alliance::BLUE] {
            if self[alliance].contains(&robot) {
                return Some(alliance);
            }
        }
        None
    }

    fn index_of(&self, robot: FtcTeamID) -> Option<MatchIndex> {
        self.alliance_of(robot).map(|alliance| {
            MatchIndex::for_match(
                self,
                alliance,
                self[alliance].iter().position(|t| *t == robot).unwrap() as u8,
            )
        })
    }
}

impl Auto<TraditionalJunction, 2, 2> for TraditionalAuto {
    type TeleOpType = TraditionalTeleOp;

    #[inline(always)]
    fn park_for(&mut self, robot: MatchIndex, location: impl Into<ParkingLocation>) {
        self.data.park(robot, location)
    }

    fn into_teleop(mut self) -> Self::TeleOpType {
        for (junction, cone_stack) in self.data.junctions.iter() {
            let points = junction.points();
            self.data.red.auto_points += (cone_stack.red_count * points) as u16;
            self.data.blue.auto_points += (cone_stack.blue_count * points) as u16;
        }
        for (alliance_info, signal_sleeves) in [
            (&mut self.data.red, self.red_signal_sleeves),
            (&mut self.data.blue, self.blue_signal_sleeves),
        ] {
            alliance_info.score_auto_parking_terminals(signal_sleeves, self.signal_zone);
        }
        unsafe { transmute(self.data) }
    }
}

macro_rules! delegated_impl {
    ($struc:ty, $delegate:tt $( , $result:literal )?) => {
        delegated_impl!($struc, $delegate, $( $result, )? (
            type BeaconErrorType = BeaconScoredOutsideEndgame;
            fn cap_for(&mut self, robot: MatchIndex, _: TraditionalJunction) -> Result<(), Self::BeaconErrorType> {
                self.$delegate.data_of_mut(robot.alliance()).beacon_placements[robot.index()] = Invalid;
                Err(BeaconScoredOutsideEndgame)
            }
        ));
    };
    ($struc:ty, $delegate:tt, $( $result:literal, )? ($( $beacon_impl:tt )+)) => {
        impl Sealed for $struc {}

        impl Index<MatchIndex> for $struc {
            type Output = FtcTeamID;

            #[inline(always)]
            fn index(&self, index: MatchIndex) -> &Self::Output {
                self.$delegate.index(index)
            }
        }

        impl Index<Alliance> for $struc {
            type Output = [FtcTeamID];

            #[inline(always)]
            fn index(&self, index: Alliance) -> &Self::Output {
                self.$delegate.index(index)
            }
        }

        impl Match<TraditionalJunction> for $struc {
            #[inline(always)]
            fn score_for(&mut self, alliance: Alliance, location: TraditionalJunction) -> bool {
                self.$delegate.score_for(alliance, location)
            }

            #[inline(always)]
            fn add_terminal_for(&mut self, alliance: Alliance, terminal: Terminal) -> bool {
                self.$delegate.add_terminal_for(alliance, terminal)
                $( ; $result )?
            }

            $( $beacon_impl )+

            type ConeRemovalErrorType = ConeRemovalError;
            fn descore(&mut self, location: TraditionalJunction) -> Result<Alliance, Self::ConeRemovalErrorType> {
                self.$delegate.descore(location)
            }

            // #[inline(always)]
            // fn descore_beacon(&mut self, location: TraditionalJunction) -> Option<MatchIndex> {
            //     self.$delegate.descore_beacon(location)
            // }

            #[inline(always)]
            fn penalize(&mut self, alliance: Alliance, points: u8) {
                self.$delegate.penalize(alliance, points)
            }

            #[inline(always)]
            fn alliance_of(&self, robot: FtcTeamID) -> Option<Alliance> {
                self.$delegate.alliance_of(robot)
            }

            #[inline(always)]
            fn index_of(&self, robot: FtcTeamID) -> Option<MatchIndex> {
                self.$delegate.index_of(robot)
            }
        }
    };
}

delegated_impl!(TraditionalAuto, data);
delegated_impl!(TraditionalTeleOp, 0, true);
delegated_impl!(TraditionalEndGame, 0, true, (
    type BeaconErrorType = BeaconError;
    #[inline(always)]
    fn cap_for(&mut self, robot: MatchIndex, location: TraditionalJunction) -> Result<(), BeaconError> {
        self.0.cap_for(robot, location)
    }
));

impl TeleOp<TraditionalJunction, 2, 2> for TraditionalTeleOp {
    type EndGameType = TraditionalEndGame;

    #[inline(always)]
    fn into_end_game(self) -> Self::EndGameType {
        unsafe { transmute(self) }
    }
}

impl TraditionalEndGame {
    fn end_match(mut self) -> [AllianceInfo<2>; 2] {
        [Alliance::RED, Alliance::BLUE].map(|alliance| {
            let internal_info = self.0.data_of(alliance);
            AllianceInfo {
                alliance,
                teams: internal_info.teams,
                penalty_points: internal_info.penalty_points,
                auto_points: internal_info.auto_points,
                teleop_points: self
                    .0
                    .junctions
                    .iter()
                    .map(|(junction, cone_stack)| {
                        (cone_stack.count(alliance) * junction.points()) as u16
                    })
                    .sum(),
                endgame_points: {
                    let mut points = 0;
                    let beacons = internal_info.beacon_placements;
                    for beacon in beacons {
                        if let Valid(junction) = beacon {
                            self.0.junctions.remove(&junction);
                            points += 10;
                        }
                    }
                    points += self
                        .0
                        .junctions
                        .values()
                        .map(|cone_stack| {
                            (alliance
                                == cone_stack
                                    .top_cone()
                                    .expect("Empty cone stacks should not exist"))
                                as u16
                                * 3
                        })
                        .sum::<u16>();
                    points
                },
            }
        })
    }
}

impl EndGame<TraditionalJunction, 2, 2> for TraditionalEndGame {
    fn park_in_terminal_for(&mut self, robot: MatchIndex) {
        // exact terminal location does not matter
        self.0.park(robot, ParkingLocation::NearTerminal)
    }

    #[inline]
    fn end_match(self) -> (AllianceInfo<2>, AllianceInfo<2>) {
        let [red, blue] = self.end_match();
        (red, blue)
    }
}
