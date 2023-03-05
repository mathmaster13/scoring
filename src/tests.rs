use crate::traditional::TraditionalAuto;
use crate::traditional::TraditionalJunction::{V1, V4, W2, W3, X2, Y1};
use crate::{Alliance, Auto, EndGame, FtcTeamID, Match, SignalZone, TeleOp};
use crate::locations::Terminal;
use crate::remote::{RedRemoteAuto, RemoteAuto, RemoteCircuitPattern, RemoteEndGame, RemoteMatch};
use crate::remote::RedRemoteJunction::{Y5, Z1, Z2, Z3, Z4, Z5};

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

#[test]
fn remote_test() {
    let mut auto = RedRemoteAuto::new(
        true,
        SignalZone::Middle,
        RemoteCircuitPattern::Pattern1
    );
    for _ in 0..5 {
        auto.score(Z2);
    }
    auto.park(SignalZone::Middle);
    auto.penalty(5);
    let mut teleop = auto.into_teleop();
    teleop.score(Z3);
    teleop.score(Z4);
    teleop.score(Z5);
    teleop.score(Y5);
    teleop.score(Z1);
    let mut endgame = teleop.into_end_game();
    endgame.add_terminal(Terminal::Near);
    endgame.add_terminal(Terminal::Far);
    endgame.park_in_terminal();
    dbg!(RemoteEndGame::end_match(endgame));
}

#[test]
fn circuit_test() {
    let mut endgame = TraditionalAuto::new(
        [true, true],
        [true, true],
        SignalZone::Middle
    ).into_teleop().into_end_game();
    endgame.add_terminal_for(Alliance::RED, Terminal::Near);
    endgame.add_terminal_for(Alliance::RED, Terminal::Far);
    endgame.score_for(Alliance::RED, Y1);
    endgame.score_for(Alliance::RED, X2);
    endgame.score_for(Alliance::RED, W3);
    endgame.score_for(Alliance::RED, V4);
    dbg!(endgame.end_match());
}