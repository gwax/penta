//! Spells and permanents cataloged for the Vintage Cube pool.

use super::search_and_reveal::stack_library;
use super::*;

/// Resolves whatever is on the stack, answering nothing.
fn resolve(game: &mut Game) {
    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
}

/// The one activation offered for `source`, if there is one.
fn activation(game: &Game, source: GameObjectId) -> Option<Action> {
    game.legal_actions(PlayerId::One).into_iter().find(
        |action| matches!(action, Action::ActivateAbility { source: id, .. } if *id == source),
    )
}

#[test]
fn the_orb_eats_a_land_for_two_life_and_nothing_else() {
    let mut game = ready_game();
    game.players[PlayerId::One.index()].life = 10;
    let orb = game
        .put_onto_battlefield(PlayerId::One, cards::ZURAN_ORB)
        .expect("cataloged");
    assert!(
        activation(&game, orb).is_none(),
        "with no land to sacrifice there is nothing to activate",
    );

    game.battlefield
        .push(creature(50_000, cards::FOREST, PlayerId::One));
    let action = activation(&game, orb).expect("a land is available to sacrifice");
    game.apply(PlayerId::One, action).expect("it is activated");
    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.definition != cards::FOREST),
        "the land is sacrificed as a cost",
    );
    resolve(&mut game);
    assert_eq!(game.players[PlayerId::One.index()].life, 12);
}

#[test]
fn the_bombardment_throws_a_creature_for_one_damage() {
    let mut game = ready_game();
    let bombardment = game
        .put_onto_battlefield(PlayerId::One, cards::GOBLIN_BOMBARDMENT)
        .expect("cataloged");
    game.battlefield
        .push(creature(50_100, cards::GRIZZLY_BEARS, PlayerId::One));
    let before = game.players[PlayerId::Two.index()].life;

    assert!(
        activation(&game, bombardment).is_some(),
        "the sacrifice ability is offered once there is a creature",
    );
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { source, targets, .. }
            if *source == bombardment
                && targets.iter().any(|selection| {
                    selection.targets().contains(&Target::Player(PlayerId::Two))
                }))
        })
        .expect("the opponent is one of the offered targets");
    game.apply(PlayerId::One, action).expect("it is activated");
    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.definition != cards::GRIZZLY_BEARS),
        "the creature is sacrificed as a cost",
    );
    resolve(&mut game);
    assert_eq!(game.players[PlayerId::Two.index()].life, before - 1);
}

#[test]
fn the_legendary_lands_count_what_they_name() {
    for (definition, color, counted, uncounted) in [
        (
            cards::GAEAS_CRADLE,
            ManaColor::Green,
            cards::GRIZZLY_BEARS,
            cards::BLACK_LOTUS,
        ),
        (
            cards::TOLARIAN_ACADEMY,
            ManaColor::Blue,
            cards::BLACK_LOTUS,
            cards::GRIZZLY_BEARS,
        ),
    ] {
        let mut game = ready_game();
        game.battlefield.clear();
        let land = game
            .put_onto_battlefield(PlayerId::One, definition)
            .expect("cataloged");
        for instance in 0..3 {
            game.battlefield
                .push(creature(50_200 + instance, counted, PlayerId::One));
        }
        game.battlefield
            .push(creature(50_300, uncounted, PlayerId::One));

        game.apply(
            PlayerId::One,
            Action::ActivateManaAbility {
                source: land,
                ability: mana_ability_for(&game, land, color),
                color,
                counters_removed: None,
                cost_object: None,
            },
        )
        .expect("the land taps for mana");
        assert_eq!(
            game.players[PlayerId::One.index()].mana_pool.amount(color),
            3,
            "{definition:?} counts only what it names",
        );
    }
}

#[test]
fn entomb_puts_the_found_card_into_the_graveyard() {
    let mut game = ready_game();
    game.players[PlayerId::One.index()].library.clear();
    stack_library(
        &mut game,
        &[(50_400, cards::SERRA_ANGEL), (50_401, cards::GRIZZLY_BEARS)],
    );
    let entomb = card(50_402, cards::ENTOMB, PlayerId::One);
    let entomb_id = entomb.id;
    game.players[PlayerId::One.index()].hand.push(entomb);
    game.players[PlayerId::One.index()].mana_pool.black = 1;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == entomb_id))
        .expect("Entomb is castable");
    game.apply(PlayerId::One, action).expect("it is cast");
    pass_priority_pair(&mut game);

    let decision = game.observe(PlayerId::One).decision.expect("a search");
    let angel = decision
        .options
        .iter()
        .find(|option| option.card.is_some_and(|(_, id)| id == cards::SERRA_ANGEL))
        .expect("every card in the library is eligible")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![angel],
        },
    )
    .expect("the search is answered");

    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::SERRA_ANGEL),
        "the found card goes to the graveyard rather than the hand",
    );
}

#[test]
fn vampiric_tutor_leaves_the_card_on_top_and_costs_two_life() {
    let mut game = ready_game();
    game.players[PlayerId::One.index()].library.clear();
    stack_library(
        &mut game,
        &[
            (50_500, cards::GRIZZLY_BEARS),
            (50_501, cards::SERRA_ANGEL),
            (50_502, cards::LIGHTNING_BOLT),
        ],
    );
    let tutor = card(50_503, cards::VAMPIRIC_TUTOR, PlayerId::One);
    let tutor_id = tutor.id;
    game.players[PlayerId::One.index()].hand.push(tutor);
    game.players[PlayerId::One.index()].mana_pool.black = 1;
    let life = game.players[PlayerId::One.index()].life;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == tutor_id))
        .expect("Vampiric Tutor is castable");
    game.apply(PlayerId::One, action).expect("it is cast");
    pass_priority_pair(&mut game);

    let decision = game.observe(PlayerId::One).decision.expect("a search");
    let angel = decision
        .options
        .iter()
        .find(|option| option.card.is_some_and(|(_, id)| id == cards::SERRA_ANGEL))
        .expect("every card in the library is eligible")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![angel],
        },
    )
    .expect("the search is answered");
    resolve(&mut game);

    assert_eq!(
        game.players[PlayerId::One.index()]
            .library
            .last()
            .map(|card| card.definition),
        Some(cards::SERRA_ANGEL),
        "the found card survives the shuffle on top",
    );
    assert_eq!(game.players[PlayerId::One.index()].life, life - 2);
}

#[test]
fn mystical_tutor_offers_only_instants_and_sorceries() {
    let mut game = ready_game();
    game.players[PlayerId::One.index()].library.clear();
    stack_library(
        &mut game,
        &[
            (50_600, cards::GRIZZLY_BEARS),
            (50_601, cards::LIGHTNING_BOLT),
            (50_602, cards::ANCESTRAL_RECALL),
        ],
    );
    let tutor = card(50_603, cards::MYSTICAL_TUTOR, PlayerId::One);
    let tutor_id = tutor.id;
    game.players[PlayerId::One.index()].hand.push(tutor);
    game.players[PlayerId::One.index()].mana_pool.blue = 1;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == tutor_id))
        .expect("Mystical Tutor is castable");
    game.apply(PlayerId::One, action).expect("it is cast");
    pass_priority_pair(&mut game);

    let decision = game.observe(PlayerId::One).decision.expect("a search");
    let mut offered = decision
        .options
        .iter()
        .filter_map(|option| option.card.map(|(_, definition)| definition))
        .collect::<Vec<_>>();
    offered.sort_unstable();
    let mut expected = vec![cards::LIGHTNING_BOLT, cards::ANCESTRAL_RECALL];
    expected.sort_unstable();
    assert_eq!(offered, expected, "the creature is not an eligible card");
}

#[test]
fn time_warp_hands_the_extra_turn_to_the_player_it_targets() {
    let mut game = ready_game();
    let warp = card(50_700, cards::TIME_WARP, PlayerId::One);
    let warp_id = warp.id;
    game.players[PlayerId::One.index()].hand.push(warp);
    game.players[PlayerId::One.index()].mana_pool.blue = 2;
    game.players[PlayerId::One.index()].mana_pool.colorless = 3;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
            if *card == warp_id
                && choices.targets().iter().any(|selection| {
                    selection.targets().contains(&Target::Player(PlayerId::Two))
                }))
        })
        .expect("Time Warp can hand the turn to the other player");
    game.apply(PlayerId::One, action).expect("it is cast");
    resolve(&mut game);

    assert_eq!(
        game.extra_turns,
        vec![PlayerId::Two],
        "the extra turn belongs to the player it targeted, not its caster",
    );
}
