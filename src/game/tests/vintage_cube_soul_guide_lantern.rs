//! Soul-Guide Lantern: one card on the way in, every opposing graveyard on
//! the way out, and a card back if neither is worth the sacrifice.

use super::*;

/// The Lantern in hand with one mana, `mine` and `theirs` already in the two
/// graveyards, and a library to draw from.
fn staged(mine: &[CardDefinitionId], theirs: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    game.players[1].graveyard.clear();
    game.players[0].library.clear();
    for (index, definition) in mine.iter().enumerate() {
        let id = 91_000 + u32::try_from(index).expect("a handful of cards");
        game.players[0]
            .graveyard
            .push(card(id, *definition, PlayerId::One));
    }
    for (index, definition) in theirs.iter().enumerate() {
        let id = 92_000 + u32::try_from(index).expect("a handful of cards");
        game.players[1]
            .graveyard
            .push(card(id, *definition, PlayerId::Two));
    }
    for index in 0..4 {
        game.players[0]
            .library
            .push(card(93_000 + index, cards::ISLAND, PlayerId::One));
    }
    let lantern = game
        .build_zone(PlayerId::One, &[cards::SOUL_GUIDE_LANTERN])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let lantern_id = lantern.id;
    game.players[0].hand.push(lantern);
    game.turns_started = [2, 1];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);
    (game, lantern_id)
}

/// Casts the Lantern and points its trigger at `victim`, stopping at the
/// target decision when none is given so the offer can be inspected.
fn cast_lantern(game: &mut Game, card: GameObjectId, victim: Option<GameObjectId>) {
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card: cast, .. } if *cast == card))
        .expect("one mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    for _ in 0..12 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let Some(victim) = victim else {
                return;
            };
            let options = decision
                .options
                .iter()
                .filter(|option| option.card.is_some_and(|(card, _)| card == victim))
                .map(|option| option.id)
                .take(1)
                .collect::<Vec<_>>();
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the choice is legal");
            continue;
        }
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

/// The Lantern once it is a permanent rather than a spell.
fn on_battlefield(game: &Game) -> GameObjectId {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::SOUL_GUIDE_LANTERN)
        .expect("the Lantern resolved")
        .card
        .id
}

/// Activates the Lantern's printed ability `index`, counting from the enters
/// trigger at zero, and lets it resolve.
fn activate(game: &mut Game, lantern: GameObjectId, index: u8) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility {
                source,
                ability: AbilityOrigin::Printed { ability, .. },
                ..
            } if *source == lantern && *ability == AbilityId(index))
        })
        .expect("the ability is offered");
    game.apply(PlayerId::One, action).expect("it is activated");
    drain_pending(game);
    game.check_state_based_actions();
}

fn offers(game: &Game, lantern: GameObjectId) -> Vec<u8> {
    let mut offered = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::ActivateAbility {
                source,
                ability: AbilityOrigin::Printed { ability, .. },
                ..
            } if source == lantern => Some(ability.0),
            _ => None,
        })
        .collect::<Vec<_>>();
    offered.sort_unstable();
    offered.dedup();
    offered
}

/// A one-mana artifact that eats a card the moment it lands.
#[test]
fn it_exiles_one_card_as_it_enters() {
    let (mut game, lantern) = staged(&[], &[cards::GRIZZLY_BEARS, cards::MOUNTAIN]);
    let bears = game.players[1].graveyard[0].id;

    cast_lantern(&mut game, lantern, Some(bears));

    assert_eq!(
        game.players[1].graveyard.len(),
        1,
        "one card taken, the other left",
    );
    assert_eq!(
        game.players[1].graveyard[0].definition,
        cards::MOUNTAIN,
        "the Bears is what left",
    );
    assert!(
        game.players[1]
            .exile
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS),
        "and it is in exile, not merely gone",
    );
}

/// "A graveyard", not "an opponent's": your own flashback card is a legal
/// target too.
#[test]
fn any_graveyard_is_a_graveyard() {
    let (mut game, lantern) = staged(&[cards::MOUNTAIN], &[cards::GRIZZLY_BEARS]);
    let mine = game.players[0].graveyard[0].id;
    let theirs = game.players[1].graveyard[0].id;

    cast_lantern(&mut game, lantern, None);

    let mut offered = game
        .pending_decisions
        .first()
        .expect("the trigger is asking")
        .observation
        .options
        .iter()
        .filter_map(|option| option.card.map(|(card, _)| card))
        .collect::<Vec<_>>();
    offered.sort_unstable();
    let mut both = vec![mine, theirs];
    both.sort_unstable();
    assert_eq!(offered, both, "either graveyard may be raided");
}

/// Cashing it in empties every opposing graveyard and leaves yours alone.
#[test]
fn the_sacrifice_clears_only_their_side() {
    let (mut game, lantern) = staged(
        &[cards::MOUNTAIN, cards::ISLAND],
        &[cards::GRIZZLY_BEARS, cards::MOUNTAIN],
    );
    let bears = game.players[1].graveyard[0].id;
    cast_lantern(&mut game, lantern, Some(bears));
    let lantern = on_battlefield(&game);

    activate(&mut game, lantern, 1);

    assert!(
        game.players[1].graveyard.is_empty(),
        "their graveyard is gone",
    );
    assert_eq!(game.players[0].graveyard.len(), 3, "yours is untouched");
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::SOUL_GUIDE_LANTERN),
        "except for the Lantern itself, which sacrificed into it",
    );
}

/// It does not need a full graveyard to be worth cashing in: the sweep is
/// untargeted, so an empty board is no obstacle.
#[test]
fn it_can_be_cashed_in_against_nothing() {
    let (mut game, lantern) = staged(&[], &[]);

    cast_lantern(&mut game, lantern, None);
    let lantern = on_battlefield(&game);

    assert_eq!(
        offers(&game, lantern),
        vec![1, 2],
        "both sacrifices are on offer with every graveyard empty",
    );
}

/// One more mana turns the Lantern into a cantrip instead.
#[test]
fn the_last_ability_draws_a_card() {
    let (mut game, lantern) = staged(&[], &[cards::GRIZZLY_BEARS]);
    let bears = game.players[1].graveyard[0].id;
    cast_lantern(&mut game, lantern, Some(bears));
    let lantern = on_battlefield(&game);
    let hand = game.players[0].hand.len();

    activate(&mut game, lantern, 2);

    assert_eq!(game.players[0].hand.len(), hand + 1, "a card for the road");
    assert!(
        game.players[1].graveyard.is_empty(),
        "the draw does not also sweep",
    );
}

/// There is only one Lantern to sacrifice, so the two abilities are a choice
/// rather than a sequence.
#[test]
fn only_one_of_the_two_sacrifices_happens() {
    let (mut game, lantern) = staged(&[], &[cards::GRIZZLY_BEARS]);
    let bears = game.players[1].graveyard[0].id;
    cast_lantern(&mut game, lantern, Some(bears));
    let lantern = on_battlefield(&game);

    activate(&mut game, lantern, 1);

    assert!(
        offers(&game, lantern).is_empty(),
        "sacrificed once, it is not there to be sacrificed again",
    );
}

/// Without the extra mana the draw is not on offer, but the sweep still is.
#[test]
fn the_draw_costs_one_more_than_the_sweep() {
    let (mut game, lantern) = staged(&[], &[]);
    cast_lantern(&mut game, lantern, None);
    let lantern = on_battlefield(&game);
    game.players[0].mana_pool = ManaPool::default();

    assert_eq!(
        offers(&game, lantern),
        vec![1],
        "the free sacrifice needs no mana; the draw needs one",
    );
}
