//! Parallax Wave: five fade counters, five creatures answered, and all of
//! them back when it goes.
//!
//! The countdown, the exile-and-return, who gets what back, and that a Wave
//! with no counters left cannot exile at all are pinned in
//! `premodern_replenish`. What is here is when the ability may be used, and
//! what shape the creature comes back in.

use super::*;

/// The Wave on the battlefield with `counters` fade counters left and
/// `theirs` creatures across the table.
fn staged(counters: u16, theirs: &[CardDefinitionId]) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let wave = game
        .put_onto_battlefield(PlayerId::One, cards::PARALLAX_WAVE)
        .expect("cataloged");
    drain_pending(&mut game);
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == wave)
    {
        permanent.set_counters(CounterKind::named("fade"), counters);
    }
    let mut ids = Vec::new();
    for definition in theirs {
        ids.push(
            game.put_onto_battlefield(PlayerId::Two, *definition)
                .expect("cataloged"),
        );
    }
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, wave, ids)
}

fn exiles(game: &Game, wave: GameObjectId, victim: GameObjectId) -> Option<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } => {
                *source == wave
                    && targets
                        .iter()
                        .any(|selection| selection.targets().contains(&Target::Permanent(victim)))
            }
            _ => false,
        })
}

/// Nothing on the ability says when: it answers a blocker in the middle of
/// their attack, which is what the card is held up for.
#[test]
fn it_exiles_at_instant_speed_on_their_turn() {
    let (mut game, wave, theirs) = staged(5, &[cards::GRIZZLY_BEARS]);
    let bears = theirs[0];
    game.active_player = PlayerId::Two;
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.declare_attacker(bears, AttackDefender::Player(PlayerId::One));
    game.finish_declaring_attackers();
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    let exile = exiles(&game, wave, bears).expect("an attacker is a creature like any other");
    game.apply(PlayerId::One, exile).expect("it activates");
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != bears),
        "the attacker is gone from the attack",
    );
    assert!(
        game.players[1]
            .exile
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS),
        "into exile, where the Wave keeps it",
    );
}

/// What comes back is a new object: the counters it was wearing and the tap
/// it was under are no part of what the Wave gives back.
#[test]
fn what_comes_back_comes_back_fresh() {
    let (mut game, wave, theirs) = staged(5, &[cards::GRIZZLY_BEARS]);
    let bears = theirs[0];
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == bears)
    {
        permanent.tapped = true;
        permanent.set_counters(CounterKind::PlusOnePlusOne, 2);
    }

    let exile = exiles(&game, wave, bears).expect("a fade counter buys the exile");
    game.apply(PlayerId::One, exile).expect("it activates");
    drain_pending(&mut game);

    game.move_permanents_to_graveyard(&[wave]);
    drain_pending(&mut game);
    game.check_state_based_actions();

    let returned = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::GRIZZLY_BEARS)
        .expect("the Wave gave it back");
    assert_eq!(
        returned.controller,
        PlayerId::Two,
        "to the player who owns it",
    );
    assert_eq!(
        returned.counters(CounterKind::PlusOnePlusOne),
        0,
        "without the counters it was wearing",
    );
    assert!(!returned.tapped, "and untapped, whatever it was before");
}
