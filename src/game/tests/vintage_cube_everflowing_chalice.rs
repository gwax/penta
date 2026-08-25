//! Everflowing Chalice: a mana rock whose size is chosen as it is cast.

use super::*;

/// The Chalice in hand with `mana` colorless available.
fn staged(mana: u16) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let chalice = game
        .build_zone(PlayerId::One, &[cards::EVERFLOWING_CHALICE])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let chalice_id = chalice.id;
    game.players[0].hand.push(chalice);
    game.turns_started = [2, 1];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, mana);
    (game, chalice_id)
}

/// Every way Player One could cast it, by how many kicks each pays.
fn kick_counts(game: &Game, chalice: GameObjectId) -> Vec<usize> {
    let mut counts: Vec<_> = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell { card, choices, .. } if card == chalice => {
                Some(choices.costs().additional().len())
            }
            _ => None,
        })
        .collect();
    counts.sort_unstable();
    counts.dedup();
    counts
}

fn cast_chalice(game: &mut Game, chalice: GameObjectId, kicks: usize) {
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == chalice && choices.costs().additional().len() == kicks
            }
            _ => false,
        })
        .unwrap_or_else(|| panic!("{kicks} kicks is payable"));
    game.apply(PlayerId::One, cast).expect("it is cast");
    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    game.check_state_based_actions();
}

fn on_battlefield(game: &Game) -> &Permanent {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::EVERFLOWING_CHALICE)
        .expect("the Chalice resolved")
}

fn charges(game: &Game) -> u16 {
    on_battlefield(game).counters(CounterKind::named("charge"))
}

fn mana_abilities(game: &Game) -> Vec<Action> {
    let chalice = on_battlefield(game).card.id;
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(
            |action| matches!(action, Action::ActivateManaAbility { source, .. } if *source == chalice),
        )
        .collect()
}

/// Multikicker is offered once for every spare {2}.
#[test]
fn every_spare_two_is_another_kick_on_offer() {
    let (game, chalice) = staged(5);

    assert_eq!(
        kick_counts(&game, chalice),
        vec![0, 1, 2],
        "free, or twice kicked out of five mana",
    );
}

/// Cast for nothing, it is a nothing: no counters and no mana.
#[test]
fn unkicked_it_taps_for_nothing() {
    let (mut game, chalice) = staged(0);

    cast_chalice(&mut game, chalice, 0);

    assert_eq!(charges(&game), 0, "no kicks, no counters");
    let before = game.players[0].mana_pool.colorless;
    let tap = mana_abilities(&game)
        .into_iter()
        .next()
        .expect("tapping it is still an action");
    game.apply(PlayerId::One, tap).expect("it taps");
    assert_eq!(
        game.players[0].mana_pool.colorless, before,
        "and it adds nothing",
    );
}

/// Each kick is a counter, and each counter is a mana every turn after.
#[test]
fn each_kick_is_a_counter_and_a_mana() {
    let (mut game, chalice) = staged(4);

    cast_chalice(&mut game, chalice, 2);

    assert_eq!(charges(&game), 2, "kicked twice");
    assert_eq!(
        game.players[0].mana_pool.colorless, 0,
        "and all four mana went into it",
    );

    let tap = mana_abilities(&game)
        .into_iter()
        .next()
        .expect("the mana ability is offered");
    game.apply(PlayerId::One, tap).expect("it taps");

    assert_eq!(
        game.players[0].mana_pool.colorless, 2,
        "two counters, two mana",
    );
    assert!(on_battlefield(&game).tapped);
}

/// The counters arrive with it rather than being put on afterwards: they are
/// there for anything watching the entry.
#[test]
fn the_counters_are_on_it_as_it_enters() {
    let (mut game, chalice) = staged(2);

    cast_chalice(&mut game, chalice, 1);

    let entered = on_battlefield(&game);
    assert_eq!(entered.counters(CounterKind::named("charge")), 1);
    assert_eq!(
        entered.entered_turn, game.turn,
        "and it is the same object that entered",
    );
}
