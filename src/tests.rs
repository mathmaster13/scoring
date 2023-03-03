use crate::traditional::TraditionalAuto;
use crate::traditional::TraditionalJunction::{V1, W2, W3};
use crate::{Alliance, Auto, EndGame, FtcTeamID, Match, SignalZone, TeleOp};

#[test]
fn test() {
    let mut auto = TraditionalAuto::from_teams(
        [(FtcTeamID(4017), true), (FtcTeamID(16145), false)],
        [(FtcTeamID(8109), true), (FtcTeamID(8110), true)],
        SignalZone::Middle,
    );
    auto.score_for(Alliance::RED, W3);
    auto.score_for(Alliance::RED, W3);
    auto.score_for(Alliance::BLUE, W3);
    auto.score_for(Alliance::RED, W3);
    auto.score_for(Alliance::BLUE, W3);
    auto.score_for(Alliance::BLUE, W2);
    auto.descore(W3).expect("TODO: panic message");
    auto.descore(W3).expect("TODO: panic message");
    auto.park_for(auto.index_of(FtcTeamID(4017)).unwrap(), SignalZone::Middle);
    let mut teleop = dbg!(auto.into_teleop());
    teleop.score_for(Alliance::BLUE, W2);
    let mut endgame = teleop.into_end_game();
    dbg!(endgame.cap_for(endgame.index_of(FtcTeamID(4017)).unwrap(), V1));
    dbg!(endgame.score_for(Alliance::RED, V1));
    dbg!(endgame.end_match());
}
