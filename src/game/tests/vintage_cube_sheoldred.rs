//! Sheoldred, the Apocalypse: the two clauses read from wherever she is
//! standing.
//!
//! That each draw is its own trigger, and that a draw which never lands
//! fires nothing, is covered with the creatures. What this adds is whose
//! draws they are, where the draw came from, and the body itself.

use super::*;

/// Sheoldred under `controller`, with both libraries stocked.
fn staged(controller: PlayerId) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    let sheoldred = creature(95_000, cards::SHEOLDRED_THE_APOCALYPSE, controller);
    let id = sheoldred.card.id;
    game.battlefield.push(sheoldred);
    for player in [PlayerId::One, PlayerId::Two] {
        game.players[player.index()].library.clear();
        for index in 0..4 {
            game.players[player.index()].library.push(card(
                95_100 + index + 10 * u32::try_from(player.index()).expect("two seats"),
                cards::GRIZZLY_BEARS,
                player,
            ));
        }
    }
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turns_started = [5, 5];
    // Past the first turn, whose draw the starting player skips.
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, id)
}

/// "Whenever you draw" and "whenever an opponent draws" are read from her
/// controller: under them, your draws are the ones that cost life.
#[test]
fn her_clauses_follow_whoever_controls_her() {
    let (mut game, _sheoldred) = staged(PlayerId::Two);
    let mine = game.players[PlayerId::One.index()].life;
    let theirs = game.players[PlayerId::Two.index()].life;

    game.draw_cards(PlayerId::One, 2);
    drain_pending(&mut game);

    assert_eq!(
        game.players[PlayerId::One.index()].life,
        mine - 4,
        "you are the opponent now",
    );
    assert_eq!(
        game.players[PlayerId::Two.index()].life,
        theirs,
        "and they gained nothing for your trouble",
    );

    game.draw_cards(PlayerId::Two, 1);
    drain_pending(&mut game);

    assert_eq!(
        game.players[PlayerId::Two.index()].life,
        theirs + 2,
        "their own draw is what pays them",
    );
}

/// The draw every turn begins with is a draw like any other: she is worth
/// two life a turn for doing nothing at all.
#[test]
fn the_draw_step_pays_her_controller() {
    let (mut game, _sheoldred) = staged(PlayerId::One);
    let mine = game.players[PlayerId::One.index()].life;
    let hand = game.players[PlayerId::One.index()].hand.len();

    game.active_player = PlayerId::One;
    game.step = Step::Upkeep;
    game.advance_step();
    drain_pending(&mut game);

    eprintln!(
        "PROBE step {:?} hand {} life {}",
        game.step,
        game.players[0].hand.len(),
        game.players[0].life
    );
    assert_eq!(game.step, Step::Draw, "the draw step is where we are");
    assert_eq!(
        game.players[PlayerId::One.index()].hand.len(),
        hand + 1,
        "a card was drawn for the turn",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].life,
        mine + 2,
        "and the turn's own draw pays her controller",
    );
}

/// The body: four power of deathtouch, which is what makes her hard to
/// attack into as well as hard to block.
#[test]
fn she_kills_what_she_touches() {
    let (mut game, sheoldred) = staged(PlayerId::One);
    let titan = game
        .put_onto_battlefield(PlayerId::Two, cards::SERRA_ANGEL)
        .expect("cataloged");
    drain_pending(&mut game);
    assert!(
        game.permanent_has_executable_keyword(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == sheoldred)
                .expect("she is there"),
            KeywordAbility::Deathtouch,
        ),
        "deathtouch",
    );

    game.active_player = PlayerId::Two;
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.declare_attacker(titan, AttackDefender::Player(PlayerId::One));
    game.finish_declaring_attackers();
    drain_pending(&mut game);
    game.declare_blocker(sheoldred, titan);
    game.deal_combat_damage();
    game.check_state_based_actions();
    drain_pending(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == titan),
        "a 4/4 blocked by four deathtouch damage is a dead 4/4",
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == sheoldred),
        "and four damage is one short of her five toughness",
    );
}
