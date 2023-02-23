use std::collections::HashMap;
use std::hint::unreachable_unchecked;
use std::mem::transmute;
use std::ops::{Index};
use super::*;
use crate::BeaconError::*;
use crate::ConeRemovalError::{BeaconOnJunction, JunctionIsEmpty};
use crate::MaybeInvalidJunction::{Invalid, Valid};

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

// possession is handled by the Match implementation
#[derive(Debug)]
struct InternalAllianceInfo<T: Junction, const N: usize> {
    teams: [FtcTeamID; N],
    penalty_points: u16,
    auto_points: u16, // aka TBP1
    terminal_amounts: (u8, u8),
    beacon_placements: [MaybeInvalidJunction<T>; N],
    parking_locations: [Option<ParkingLocation>; N]
}

impl <T: Junction, const N: usize> InternalAllianceInfo<T, N> {
    fn new(teams: [FtcTeamID; N]) -> Self {
        Self {
            teams,
            penalty_points: 0,
            auto_points: 0,
            terminal_amounts: (0, 0),
            beacon_placements: [MaybeInvalidJunction::None; N],
            parking_locations: [None; N]
        }
    }
}

#[derive(Debug)]
struct InternalQualificationMatchAuto {
    red: InternalAllianceInfo<TraditionalJunction, 2>,
    blue: InternalAllianceInfo<TraditionalJunction, 2>,
    // beacons are not stored here!
    junctions: HashMap<TraditionalJunction, ConeStack>
}

// like has_beacon_on, but inlined to appease the borrow checker
macro_rules! has_beacon_on {
    ($this:ident, $location:ident) => {
        $this.red.beacon_placements.contains(&Valid($location)) ||
            $this.blue.beacon_placements.contains(&Valid($location))
    };
}

impl InternalQualificationMatchAuto {
    fn data_of(&self, index: Alliance) -> &InternalAllianceInfo<TraditionalJunction, 2> {
        match index {
            Alliance::RED => &self.red,
            Alliance::BLUE => &self.blue
        }
    }

    fn data_of_mut(&mut self, index: Alliance) -> &mut InternalAllianceInfo<TraditionalJunction, 2> {
        match index {
            Alliance::RED => &mut self.red,
            Alliance::BLUE => &mut self.blue
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
                junctions: HashMap::new()
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

    fn park(&mut self, robot: MatchIndex, location: impl Into<ParkingLocation>) -> bool {
        self.data_of_mut(robot.alliance()).parking_locations[robot.index()] = Some(location.into());
        true
    }
}

#[derive(Debug)]
pub struct QualificationMatchAuto {
    data: InternalQualificationMatchAuto,
    red_signal_sleeves: [bool; 2],
    blue_signal_sleeves: [bool; 2],
    signal_zone: SignalZone
}

impl QualificationMatchAuto {
    pub fn new(red: [(FtcTeamID, bool); 2], blue: [(FtcTeamID, bool); 2], signal_zone: SignalZone) -> Self {
        let [(red1, red1sleeve), (red2, red2sleeve)] = red;
        let [(blue1, blue1sleeve), (blue2, blue2sleeve)] = blue;
        Self {
            data: InternalQualificationMatchAuto::new([red1, red2], [blue1, blue2], false),
            red_signal_sleeves: [red1sleeve, red2sleeve],
            blue_signal_sleeves: [blue1sleeve, blue2sleeve],
            signal_zone
        }
    }

    pub fn try_new(red: [(FtcTeamID, bool); 2], blue: [(FtcTeamID, bool); 2], signal_zone: SignalZone) -> Option<Self> {
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
}

#[repr(transparent)]
#[derive(Debug)]
pub struct QualificationMatchTeleOp(InternalQualificationMatchAuto);

#[repr(transparent)]
#[derive(Debug)]
pub struct QualificationMatchEndGame(InternalQualificationMatchAuto);

impl Index<MatchIndex> for InternalQualificationMatchAuto {
    type Output = FtcTeamID;

    fn index(&self, index: MatchIndex) -> &Self::Output {
        &self.data_of(index.alliance()).teams[index.index()]
    }
}

impl Index<Alliance> for InternalQualificationMatchAuto {
    type Output = [FtcTeamID; 2];

    fn index(&self, index: Alliance) -> &Self::Output {
        &self.data_of(index).teams
    }
}

impl Match<TraditionalJunction, 2> for InternalQualificationMatchAuto {
    // returns true if modification was successful
    fn add_cone(&mut self, alliance: Alliance, location: TraditionalJunction) -> bool {
        if self.has_beacon_on(location) { return false; }
        match self.junctions.get_mut(&location) {
            Some(cone_stack) => {
                cone_stack.push(alliance);
            },
            None => {
                self.junctions.insert(location, ConeStack::new(alliance));
            }
        }
        true
    }

    type ConeRemovalErrorType = ConeRemovalError;
    fn remove_cone(&mut self, location: TraditionalJunction) -> Result<Alliance, Self::ConeRemovalErrorType> {
        match self.junctions.get_mut(&location) {
            Some(cone_stack) => {
                if has_beacon_on!(self, location) {
                    Err(BeaconOnJunction)
                } else {
                    let (alliance, is_empty) = cone_stack.pop();
                    if is_empty {
                        self.junctions.remove(&location);
                    }
                    Ok(alliance)
                }
            }
            None => Err(JunctionIsEmpty)
        }
    }

    fn add_terminal(&mut self, alliance: Alliance, terminal: Terminal) -> bool {
        let amounts = &mut self.data_of_mut(alliance).terminal_amounts;
        let near_terminal = terminal == Terminal::Near;
        if near_terminal {
            amounts.0 += 1;
        } else {
            amounts.1 += 1;
        }
        near_terminal
    }

    type BeaconErrorType = BeaconError;
    fn add_beacon(&mut self, robot: MatchIndex, location: TraditionalJunction) -> Result<(), Self::BeaconErrorType> {
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

        if alliance_info.beacon_placements[team_index] != MaybeInvalidJunction::None { return Err(BeaconPreviouslyScored); }
        alliance_info.beacon_placements[team_index] = match output {
            Ok(()) => Valid(location),
            Err(JunctionIsCapped) => Invalid,
            _ => unsafe { unreachable_unchecked() }
        };
        output
    }

    fn penalty(&mut self, alliance: Alliance, points: u8) {
        self.data_of_mut(alliance).penalty_points += points as u16;
    }

    fn alliance_of(&self, robot: FtcTeamID) -> Option<Alliance> {
        for alliance in [Alliance::RED, Alliance::BLUE] {
            if self[alliance].contains(&robot) { return Some(alliance); }
        }
        None
    }

    fn index_of(&self, robot: FtcTeamID) -> Option<MatchIndex> {
        self.alliance_of(robot).map(|alliance|
            MatchIndex::new(
                alliance,
                self[alliance].iter().position(|t| *t == robot).unwrap()
            )
        )
    }
}

impl Auto<TraditionalJunction, 2> for QualificationMatchAuto {
    type TeleOpType = QualificationMatchTeleOp;

    #[inline(always)]
    fn park(&mut self, robot: MatchIndex, location: impl Into<ParkingLocation>) -> bool {
        self.data.park(robot, location)
    }

    fn into_teleop(mut self) -> Self::TeleOpType {
        for (junction, cone_stack) in self.data.junctions.iter() {
            let points = junction.points();
            self.data.red.auto_points += (cone_stack.red_count * points) as u16;
            self.data.blue.auto_points += (cone_stack.blue_count * points) as u16;
        }
        let signal_zone = self.signal_zone;
        for (mut alliance_info, signal_sleeves) in [(&mut self.data.red, self.red_signal_sleeves), (&mut self.data.blue, self.blue_signal_sleeves)] {
            alliance_info.auto_points += {
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
            } as u16;
            alliance_info.parking_locations = [None; 2];
        }
        unsafe { transmute(self.data) }
    }
}

macro_rules! delegated_impl {
    ($struc:ty, $delegate:tt, $beacon_impl:item $(, $result:literal)?) => {
        impl Index<MatchIndex> for $struc {
            type Output = FtcTeamID;

            #[inline(always)]
            fn index(&self, index: MatchIndex) -> &Self::Output {
                self.$delegate.index(index)
            }
        }

        impl Index<Alliance> for $struc {
            type Output = [FtcTeamID; 2];

            #[inline(always)]
            fn index(&self, index: Alliance) -> &Self::Output {
                self.$delegate.index(index)
            }
        }

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

            type ConeRemovalErrorType = ConeRemovalError;
            fn remove_cone(&mut self, location: TraditionalJunction) -> Result<Alliance, Self::ConeRemovalErrorType> {
                self.$delegate.remove_cone(location)
            }

            #[inline(always)]
            fn penalty(&mut self, alliance: Alliance, points: u8) {
                self.$delegate.penalty(alliance, points)
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

macro_rules! beacon {
    ($delegate:tt) => {
        fn add_beacon(&mut self, robot: MatchIndex, _: TraditionalJunction) -> Result<(), BeaconError> {
            self.$delegate.data_of_mut(robot.alliance()).beacon_placements[robot.index()] = Invalid;
            Err(ScoredOutsideEndgame)
        }
    };
}

delegated_impl!(QualificationMatchAuto, data, beacon! { data });
delegated_impl!(QualificationMatchTeleOp, 0, beacon! { 0 }, true);
delegated_impl!(QualificationMatchEndGame, 0,
    fn add_beacon(&mut self, robot: MatchIndex, location: TraditionalJunction) -> Result<(), BeaconError> {
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
    fn park_in_terminal(&mut self, robot: MatchIndex) -> bool {
        // exact terminal location does not matter
        self.0.park(robot, ParkingLocation::NearTerminal)
    }

    fn end_match(self) -> [AllianceInfo<2>; 2] {
        [Alliance::RED, Alliance::BLUE].map(|alliance| {
            let internal_info = self.0.data_of(alliance);
            AllianceInfo {
                alliance,
                teams: internal_info.teams,
                penalty_points: internal_info.penalty_points,
                auto_points: internal_info.auto_points,
                teleop_points: self.0.junctions.iter()
                    .map(|(junction, cone_stack)| (
                        cone_stack.count(alliance) * junction.points()) as u16
                    ).sum(),
                endgame_points: {
                    let mut points = 0;
                    let beacons = internal_info.beacon_placements;
                    for beacon in beacons {
                        if let Valid(_) = beacon {
                            points += 10;
                        }
                    }
                    points += self.0.junctions.iter()
                        .map(|(&junction, cone_stack)|
                             (alliance == cone_stack.top_cone() && !beacons.contains(&Valid(junction))) as u16 * 3
                        ).sum::<u16>();
                    points
                },
            }
        })
    }
}
