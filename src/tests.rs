use crate::{Alliance, Auto, EndGame, FtcTeamID, Match, SignalZone, TeleOp};
use crate::traditional::QualificationMatchAuto;
use crate::traditional::TraditionalJunction::{V1, W2, W3};

#[test]
fn test() {
    let mut auto = QualificationMatchAuto::new(
        [(FtcTeamID(4017), true), (FtcTeamID(16145), false)],
        [(FtcTeamID(8109), true), (FtcTeamID(8110), true)],
        SignalZone::Middle
    );
    auto.add_cone(Alliance::RED, W3);
    auto.add_cone(Alliance::RED, W3);
    auto.add_cone(Alliance::BLUE, W3);
    auto.add_cone(Alliance::RED, W3);
    auto.add_cone(Alliance::BLUE, W3);
    auto.add_cone(Alliance::BLUE, W2);
    auto.remove_cone(W3).expect("TODO: panic message");
    auto.remove_cone(W3).expect("TODO: panic message");
    auto.park(auto.index_of(FtcTeamID(4017)).unwrap(), SignalZone::Middle);
    let mut teleop = dbg!(auto.into_teleop());
    teleop.add_cone(Alliance::BLUE, W2);
    let mut endgame = teleop.into_end_game();
    dbg!(endgame.add_beacon(endgame.index_of(FtcTeamID(4017)).unwrap(), V1));
    dbg!(endgame.add_cone(Alliance::RED, V1));
    dbg!(endgame.end_match());
}