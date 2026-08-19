//! Creatures cataloged for the Vintage Cube pool.

use super::search_and_reveal::stack_library;
use super::*;

/// The Ent as a spell: six mana for a body that brings a Food with it.
#[test]
fn the_ent_arrives_with_reach_and_a_food_token() {
    let mut game = ready_game();
    game.battlefield.clear();
    let ent = game
        .put_onto_battlefield(PlayerId::One, cards::GENEROUS_ENT)
        .expect("cataloged");
    drain_pending(&mut game);

    let ent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == ent)
        .expect("it entered");
    assert_eq!((game.power(ent), game.toughness(ent)), (Some(5), Some(7)));
    assert!(
        game.permanent_has_executable_keyword(ent, KeywordAbility::Reach),
        "a Treefolk this size blocks fliers",
    );

    let food = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::FOOD_TOKEN)
        .expect("the enters trigger made a Food");
    let rules = game.effective_rules(food).expect("the token has rules");
    assert!(
        rules.has_subtype("Food"),
        "Food is an artifact type, not a creature type",
    );
    assert!(rules.has_type(crate::card::CardType::Artifact));
    assert!(!rules.has_type(crate::card::CardType::Creature));
}

/// The Food it left behind: three life for two mana and itself.
#[test]
fn the_food_token_is_eaten_for_three_life() {
    let mut game = ready_game();
    game.battlefield.clear();
    let food = game
        .put_onto_battlefield(PlayerId::One, cards::FOOD_TOKEN)
        .expect("cataloged");
    game.players[PlayerId::One.index()].life = 10;
    game.players[PlayerId::One.index()].mana_pool.colorless = 2;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == food))
        .expect("the Food can be eaten");
    game.apply(PlayerId::One, action).expect("it is activated");
    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != food),
        "sacrificing it is a cost",
    );
    drain_pending(&mut game);
    assert_eq!(game.players[PlayerId::One.index()].life, 13);
}

/// The Ent as a land: one mana from hand, and it fetches a Forest instead of
/// drawing. Anything with the Forest subtype counts, not just the basic.
#[test]
fn forestcycling_finds_a_forest_rather_than_drawing() {
    let mut game = ready_game();
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::One.index()].library.clear();
    stack_library(
        &mut game,
        &[
            (52_000, cards::LIGHTNING_BOLT),
            (52_001, cards::TAIGA),
            (52_002, cards::ISLAND),
        ],
    );
    let ent = card(52_003, cards::GENEROUS_ENT, PlayerId::One);
    let ent_id = ent.id;
    game.players[PlayerId::One.index()].hand.push(ent);
    game.players[PlayerId::One.index()].mana_pool.colorless = 1;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == ent_id),
        )
        .expect("forestcycling is offered from hand");
    game.apply(PlayerId::One, action).expect("it is activated");
    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::GENEROUS_ENT),
        "the discard is a cost",
    );
    pass_priority_pair(&mut game);

    let decision = game.observe(PlayerId::One).decision.expect("a search");
    assert_eq!(
        decision
            .options
            .iter()
            .filter_map(|option| option.card.map(|(_, definition)| definition))
            .collect::<Vec<_>>(),
        vec![cards::TAIGA],
        "a dual land is a Forest; the Island and the Bolt are not",
    );
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![decision.options[0].id],
        },
    )
    .expect("the search is answered");

    assert!(
        game.players[PlayerId::One.index()]
            .hand
            .iter()
            .any(|card| card.definition == cards::TAIGA),
        "the found land goes to hand rather than the battlefield",
    );
}

/// The Titan's ability is one ability with two ways in. Both paths reach the
/// same search, and the search takes any land rather than only basics.
#[test]
fn the_titan_fetches_on_entering_and_again_on_attacking() {
    for attack_instead in [false, true] {
        let mut game = ready_game();
        game.battlefield.clear();
        game.players[PlayerId::One.index()].library.clear();
        stack_library(
            &mut game,
            &[
                (58_000, cards::TAIGA),
                (58_001, cards::FOREST),
                (58_002, cards::LIGHTNING_BOLT),
            ],
        );

        let titan = if attack_instead {
            // Already here since last turn, so it can attack.
            let titan = creature(58_100, cards::PRIMEVAL_TITAN, PlayerId::One);
            let id = titan.card.id;
            game.battlefield.push(titan);
            id
        } else {
            game.put_onto_battlefield(PlayerId::One, cards::PRIMEVAL_TITAN)
                .expect("cataloged")
        };

        if attack_instead {
            game.step = Step::DeclareAttackers;
            game.declare_attacker(titan, AttackDefender::Player(PlayerId::Two));
            game.finish_declaring_attackers();
        }

        // The search is optional, so answering it is what takes the lands.
        let decision = loop {
            if let Some(decision) = game.observe(PlayerId::One).decision {
                break decision;
            }
            let player = game.priority;
            game.apply(player, Action::PassPriority)
                .expect("the trigger is on the stack");
        };
        let accept = decision
            .options
            .last()
            .expect("the optional search offers accepting it")
            .id;
        game.apply(
            PlayerId::One,
            Action::ChooseDecision {
                decision: decision.id,
                options: vec![accept],
            },
        )
        .expect("the search is accepted");

        let search = game
            .observe(PlayerId::One)
            .decision
            .expect("the library search follows");
        let mut offered = search
            .options
            .iter()
            .filter_map(|option| option.card.map(|(_, definition)| definition))
            .collect::<Vec<_>>();
        offered.sort_unstable();
        let mut lands = vec![cards::TAIGA, cards::FOREST];
        lands.sort_unstable();
        assert_eq!(
            offered, lands,
            "any land card, and nothing that is not a land (attacking: {attack_instead})",
        );
        let chosen = search
            .options
            .iter()
            .map(|option| option.id)
            .collect::<Vec<_>>();
        game.apply(
            PlayerId::One,
            Action::ChooseDecision {
                decision: search.id,
                options: chosen,
            },
        )
        .expect("both lands are taken");

        for land in [cards::TAIGA, cards::FOREST] {
            let found = game
                .battlefield
                .iter()
                .find(|permanent| permanent.card.definition == land)
                .unwrap_or_else(|| panic!("{land:?} arrived"));
            assert!(found.tapped, "the lands arrive tapped");
        }
    }
}
