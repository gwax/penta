//! Recurring Nightmare: a creature for a creature, every turn, with the
//! enchantment coming back to hand to do it again.

use super::*;

/// The Nightmare on the battlefield, `mine` under Player One, and `buried`
/// in Player One's graveyard.
fn staged(mine: &[CardDefinitionId], buried: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    let nightmare = game
        .put_onto_battlefield(PlayerId::One, cards::RECURRING_NIGHTMARE)
        .expect("cataloged");
    for definition in mine {
        game.put_onto_battlefield(PlayerId::One, *definition)
            .expect("cataloged");
    }
    for (index, definition) in buried.iter().enumerate() {
        let id = 99_000 + u32::try_from(index).expect("a handful of cards");
        game.players[0]
            .graveyard
            .push(card(id, *definition, PlayerId::One));
    }
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turns_started = [4, 4];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, nightmare)
}

/// Every activation the Nightmare offers, as (sacrificed, reanimated).
fn offers(game: &Game, nightmare: GameObjectId) -> Vec<(GameObjectId, GameObjectId)> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::ActivateAbility {
                source,
                targets,
                cost_objects,
                ..
            } if source == nightmare => {
                let target = targets.iter().find_map(|selection| {
                    selection.targets().iter().find_map(|target| match target {
                        Target::Card(id) | Target::Permanent(id) => Some(*id),
                        _ => None,
                    })
                })?;
                Some((*cost_objects.first()?, target))
            }
            _ => None,
        })
        .collect()
}

/// Activates the Nightmare, sacrificing `victim` to reanimate `wanted`.
fn activate(game: &mut Game, nightmare: GameObjectId, victim: GameObjectId, wanted: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source,
                targets,
                cost_objects,
                ..
            } => {
                *source == nightmare
                    && cost_objects.contains(&victim)
                    && targets.iter().any(|selection| {
                        selection
                            .targets()
                            .iter()
                            .any(|target| matches!(target, Target::Card(id) if *id == wanted))
                    })
            }
            _ => false,
        })
        .expect("that pairing is offered");
    game.apply(PlayerId::One, action).expect("it activates");
    drain_pending(game);
    game.check_state_based_actions();
}

fn on_battlefield(game: &Game, definition: CardDefinitionId) -> usize {
    game.battlefield
        .iter()
        .filter(|permanent| permanent.card.definition == definition)
        .count()
}

/// One creature for another, and the enchantment back in hand to do it
/// again next turn.
#[test]
fn it_trades_a_creature_for_a_better_one() {
    let (mut game, nightmare) = staged(&[cards::GRIZZLY_BEARS], &[cards::SERRA_ANGEL]);
    let bears = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::GRIZZLY_BEARS)
        .expect("the Bear is there")
        .card
        .id;
    let angel = game.players[0].graveyard[0].id;

    activate(&mut game, nightmare, bears, angel);

    assert_eq!(
        on_battlefield(&game, cards::SERRA_ANGEL),
        1,
        "the Angel is back"
    );
    assert_eq!(
        on_battlefield(&game, cards::GRIZZLY_BEARS),
        0,
        "the Bear paid"
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS),
        "and the Bear is where the Angel was",
    );
    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::RECURRING_NIGHTMARE),
        "the enchantment came back to hand rather than dying",
    );
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == nightmare),
        "so it is no longer on the battlefield",
    );
}

/// Targets are chosen as the ability is activated and costs are paid after
/// (CR 601.2), so the creature being sacrificed cannot be the one coming
/// back: it is still on the battlefield when the target is named.
#[test]
fn the_sacrificed_creature_is_never_the_target() {
    let (game, nightmare) = staged(&[cards::GRIZZLY_BEARS], &[cards::SERRA_ANGEL]);

    let offered = offers(&game, nightmare);
    assert!(!offered.is_empty(), "there is something to do");
    assert!(
        offered
            .iter()
            .all(|(sacrificed, reanimated)| sacrificed != reanimated),
        "no activation both sacrifices and returns one creature",
    );
}

/// Both halves of the cost are needed: with no creature to sacrifice there
/// is no activation, however full the graveyard is.
#[test]
fn it_needs_a_creature_to_feed_it() {
    let (game, nightmare) = staged(&[], &[cards::SERRA_ANGEL]);

    assert!(
        offers(&game, nightmare).is_empty(),
        "nothing to sacrifice is nothing to activate",
    );
}

/// And a creature card to bring back: an empty graveyard leaves the ability
/// with no legal target.
#[test]
fn it_needs_something_worth_bringing_back() {
    let (game, nightmare) = staged(&[cards::GRIZZLY_BEARS], &[]);

    assert!(
        offers(&game, nightmare).is_empty(),
        "an empty graveyard has no legal target",
    );
}

/// A noncreature card in the graveyard is not a creature card.
#[test]
fn it_only_reads_creature_cards() {
    let (game, nightmare) = staged(&[cards::GRIZZLY_BEARS], &[cards::LIGHTNING_BOLT]);

    assert!(
        offers(&game, nightmare).is_empty(),
        "a Bolt in the graveyard is not a body",
    );
}

/// "Activate only as a sorcery": not on their turn, and not in response.
#[test]
fn it_waits_for_a_sorcery_window() {
    let (mut game, nightmare) = staged(&[cards::GRIZZLY_BEARS], &[cards::SERRA_ANGEL]);
    game.active_player = PlayerId::Two;

    assert!(
        offers(&game, nightmare).is_empty(),
        "their turn is not your sorcery window",
    );

    game.active_player = PlayerId::One;
    game.step = Step::EndOfCombat;
    assert!(
        offers(&game, nightmare).is_empty(),
        "and neither is the end of combat",
    );

    game.step = Step::PostcombatMain;
    assert!(!offers(&game, nightmare).is_empty(), "either main phase is");
}

/// Cast it in your main phase and it is yours to use at once: you hold
/// priority as it resolves, so the ability is live on the turn it entered
/// and before anybody can answer the enchantment. The creature it eats may
/// be just as new -- sacrificing does not care about summoning sickness.
#[test]
fn it_can_be_used_the_turn_it_is_cast() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    game.turns_started = [4, 4];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.players[0]
        .graveyard
        .push(card(99_100, cards::SERRA_ANGEL, PlayerId::One));
    let bears = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);

    let nightmare = card(99_101, cards::RECURRING_NIGHTMARE, PlayerId::One);
    let nightmare_id = nightmare.id;
    game.players[0].hand.push(nightmare);
    let pool = &mut game.players[0].mana_pool;
    pool.black = 1;
    pool.colorless = 2;
    game.apply(
        PlayerId::One,
        cast_action(nightmare_id, Vec::new(), Vec::new(), 0),
    )
    .expect("three mana buys it");
    drain_pending(&mut game);

    let entered = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::RECURRING_NIGHTMARE)
        .expect("it resolved");
    assert_eq!(
        entered.entered_controller_turn, game.turns_started[0],
        "it entered this turn",
    );
    let nightmare = entered.card.id;

    let angel = game.players[0].graveyard[0].id;
    activate(&mut game, nightmare, bears, angel);

    assert_eq!(
        on_battlefield(&game, cards::SERRA_ANGEL),
        1,
        "the Angel came back on the same turn the Nightmare arrived",
    );
    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::RECURRING_NIGHTMARE),
        "and the enchantment is back in hand before anybody could answer it",
    );
}
