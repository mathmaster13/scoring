use crate::{Alliance, Auto, FtcTeamID, Match, QualificationMatchAuto, SignalZone, TraditionalJunction};
use crate::Alliance::BLUE;
use crate::ParkingLocation::{LeftSignalZone, MiddleSignalZone, NearTerminal, Substation};
use crate::Terminal::Far;

#[test]
fn test() {
    let mut auto = QualificationMatchAuto::new(
        [(FtcTeamID(4017), true), (FtcTeamID(16145), false)],
        [(FtcTeamID(-1), false), (FtcTeamID(-2), false)],
        SignalZone::Middle
    );
    for _ in 0..10 {
        auto.add_cone(Alliance::RED, TraditionalJunction::W3);
    }
    dbg!(auto.park(FtcTeamID(4017), Substation));
    dbg!(auto.alliance_of(FtcTeamID(4017)));
    dbg!(auto.add_beacon(FtcTeamID(16145), TraditionalJunction::W2));
    dbg!(auto.add_terminal(BLUE, Far));
    println!("{:?}", auto.into_teleop().0.red);
}