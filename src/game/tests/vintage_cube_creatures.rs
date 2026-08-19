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

/// Cecil's front half turns the damage it deals into life loss, and the same
/// clause checks afterwards whether that loss has taken its controller low
/// enough to turn the card over.
#[test]
fn cecil_transforms_once_his_own_damage_has_halved_your_life() {
    for (starting_life, transforms) in [(20, false), (13, true)] {
        let mut game = ready_game();
        game.battlefield.clear();
        game.players[PlayerId::One.index()].life = starting_life;
        let cecil = creature(59_000, cards::CECIL_DARK_KNIGHT, PlayerId::One);
        let cecil_id = cecil.card.id;
        game.battlefield.push(cecil);
        game.tap_permanent(cecil_id);

        game.damage_target_from(Some(cecil_id), Some(Target::Player(PlayerId::Two)), 3);
        drain_pending(&mut game);

        assert_eq!(
            game.players[PlayerId::One.index()].life,
            starting_life - 3,
            "the damage Cecil dealt is repaid in life",
        );
        let permanent = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == cecil_id)
            .expect("he is still here");
        assert_eq!(
            permanent.presented == CardPartId(1),
            transforms,
            "at {starting_life} life, three damage should{} turn him over",
            if transforms { "" } else { " not" },
        );
        if transforms {
            assert!(!permanent.tapped, "and untap him on the way");
            assert_eq!(
                (game.power(permanent), game.toughness(permanent)),
                (Some(4), Some(4)),
                "the back half is the bigger one",
            );
        }
    }
}

/// The back half protects the rest of the attack, and not itself: "other
/// attacking creatures" is the whole clause.
#[test]
fn the_redeemed_paladin_covers_the_other_attackers() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].life = 5;
    let cecil = creature(59_100, cards::CECIL_DARK_KNIGHT, PlayerId::One);
    let cecil_id = cecil.card.id;
    game.battlefield.push(cecil);
    // Halve his controller's life with his own damage to get the back face.
    game.damage_target_from(Some(cecil_id), Some(Target::Player(PlayerId::Two)), 1);
    drain_pending(&mut game);
    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == cecil_id)
            .expect("still here")
            .presented,
        CardPartId(1),
        "the Paladin side is up",
    );

    let friend = creature(59_101, cards::GRIZZLY_BEARS, PlayerId::One);
    let friend_id = friend.card.id;
    game.battlefield.push(friend);
    let bystander = creature(59_102, cards::SAVANNAH_LIONS, PlayerId::One);
    let bystander_id = bystander.card.id;
    game.battlefield.push(bystander);

    game.step = Step::DeclareAttackers;
    game.declare_attacker(cecil_id, AttackDefender::Player(PlayerId::Two));
    game.declare_attacker(friend_id, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    drain_pending(&mut game);

    let indestructible = |id| {
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == id)
            .is_some_and(|permanent| game.has_indestructible(permanent))
    };
    assert!(indestructible(friend_id), "the other attacker is covered");
    assert!(
        !indestructible(bystander_id),
        "a creature that stayed home is not attacking",
    );
    assert!(
        !indestructible(cecil_id),
        "and \"other\" excludes Cecil himself",
    );
}

/// A Lhurgoyf counts card types, not cards: a graveyard of ten creatures is
/// worth the same as a graveyard of one.
#[test]
fn pyrogoyf_grows_with_the_types_in_every_graveyard_not_the_cards() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].graveyard.clear();
    game.players[PlayerId::Two.index()].graveyard.clear();
    let goyf = game
        .put_onto_battlefield(PlayerId::One, cards::PYROGOYF)
        .expect("cataloged");
    let stats = |game: &Game| {
        let permanent = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == goyf)
            .expect("still there");
        (game.power(permanent), game.toughness(permanent))
    };
    assert_eq!(
        stats(&game),
        (Some(0), Some(1)),
        "an empty graveyard is 0/1"
    );

    // Three creature cards are still one type.
    for instance in 0..3 {
        game.players[PlayerId::One.index()].graveyard.push(card(
            62_000 + instance,
            cards::GRIZZLY_BEARS,
            PlayerId::One,
        ));
    }
    assert_eq!(
        stats(&game),
        (Some(1), Some(2)),
        "one type among three cards"
    );

    // An instant in the same graveyard is a second type.
    game.players[PlayerId::One.index()].graveyard.push(card(
        62_100,
        cards::LIGHTNING_BOLT,
        PlayerId::One,
    ));
    assert_eq!(stats(&game), (Some(2), Some(3)));

    // "All graveyards" reaches across the table.
    game.players[PlayerId::Two.index()]
        .graveyard
        .push(card(62_200, cards::FOREST, PlayerId::Two));
    assert_eq!(
        stats(&game),
        (Some(3), Some(4)),
        "the opponent's land counts"
    );

    // A second instant adds nothing, because the type is already there.
    game.players[PlayerId::Two.index()].graveyard.push(card(
        62_300,
        cards::ANCESTRAL_RECALL,
        PlayerId::Two,
    ));
    assert_eq!(stats(&game), (Some(3), Some(4)));
}

/// Its own arrival is what usually triggers it, and the damage is read from
/// the creature that entered.
#[test]
fn pyrogoyf_burns_on_arrival_for_as_much_as_it_is_worth() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].graveyard.clear();
    game.players[PlayerId::Two.index()].graveyard.clear();
    // Two types in the graveyard: a 2/3 arriving.
    game.players[PlayerId::One.index()].graveyard.push(card(
        62_400,
        cards::GRIZZLY_BEARS,
        PlayerId::One,
    ));
    game.players[PlayerId::One.index()].graveyard.push(card(
        62_401,
        cards::LIGHTNING_BOLT,
        PlayerId::One,
    ));
    let before = game.players[PlayerId::Two.index()].life;

    game.put_onto_battlefield(PlayerId::One, cards::PYROGOYF)
        .expect("cataloged");

    let decision = loop {
        if let Some(decision) = game.observe(PlayerId::One).decision {
            break decision;
        }
        let player = game.priority;
        assert!(
            game.apply(player, Action::PassPriority).is_ok(),
            "the enters trigger should be waiting on a target",
        );
    };
    let opponent = decision
        .options
        .iter()
        .find(|option| option.label == "your opponent")
        .expect("the opponent is one of the offered targets")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![opponent],
        },
    )
    .expect("a target is chosen");
    drain_pending(&mut game);

    assert_eq!(
        before - game.players[PlayerId::Two.index()].life,
        2,
        "a 2/3 Lhurgoyf burns for two",
    );
}

/// The Krasis lends its own body: a 1/1 becomes a 4/4 while the Krasis is a
/// 4/4, and a 7/7 while it is a 7/7. Setting a base rather than adding to one
/// is what makes the small creature big rather than bigger.
#[test]
fn the_krasis_sets_another_creature_to_its_own_size() {
    for (adapted, expected) in [(false, 4), (true, 7)] {
        let mut game = ready_game();
        game.battlefield.clear();
        let krasis = creature(64_000, cards::UNRULY_KRASIS, PlayerId::One);
        let krasis_id = krasis.card.id;
        game.battlefield.push(krasis);
        let lions = creature(64_001, cards::SAVANNAH_LIONS, PlayerId::One);
        let lions_id = lions.card.id;
        game.battlefield.push(lions);

        if adapted {
            game.players[PlayerId::One.index()].mana_pool.green = 1;
            game.players[PlayerId::One.index()].mana_pool.blue = 1;
            game.players[PlayerId::One.index()].mana_pool.colorless = 3;
            let adapt = game
                .legal_actions(PlayerId::One)
                .into_iter()
                .find(|action| {
                    matches!(action, Action::ActivateAbility { source, .. } if *source == krasis_id)
                })
                .expect("adapt is offered");
            game.apply(PlayerId::One, adapt).expect("it activates");
            drain_pending(&mut game);
        }

        game.step = Step::DeclareAttackers;
        game.declare_attacker(krasis_id, AttackDefender::Player(PlayerId::Two));
        game.finish_declaring_attackers();

        // Two answers follow in order: which creature the trigger targets,
        // and then whether to take the optional effect at all. The last
        // option is the affirmative one in both.
        for _ in 0..16 {
            if let Some(decision) = game.observe(PlayerId::One).decision {
                let accept = decision.options.last().expect("an option is offered").id;
                game.apply(
                    PlayerId::One,
                    Action::ChooseDecision {
                        decision: decision.id,
                        options: vec![accept],
                    },
                )
                .expect("the offered option is legal");
                continue;
            }
            if game.stack.is_empty() && game.pending_triggers.is_empty() {
                break;
            }
            let player = game.priority;
            assert!(
                game.apply(player, Action::PassPriority).is_ok(),
                "the attack trigger is waiting",
            );
        }

        let lions = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == lions_id)
            .expect("still there");
        assert_eq!(
            (game.power(lions), game.toughness(lions)),
            (Some(expected), Some(expected)),
            "a 2/1 takes the Krasis's size (adapted: {adapted})",
        );
    }
}

/// Adapt is a conditional rather than a cost: the second activation resolves
/// and simply finds counters already there.
#[test]
fn the_krasis_adapts_only_while_it_has_no_counters() {
    let mut game = ready_game();
    game.battlefield.clear();
    let krasis = creature(64_100, cards::UNRULY_KRASIS, PlayerId::One);
    let krasis_id = krasis.card.id;
    game.battlefield.push(krasis);

    let activate = |game: &mut Game| {
        game.players[PlayerId::One.index()].mana_pool.green = 1;
        game.players[PlayerId::One.index()].mana_pool.blue = 1;
        game.players[PlayerId::One.index()].mana_pool.colorless = 3;
        let action = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| {
                matches!(action, Action::ActivateAbility { source, .. } if *source == krasis_id)
            })
            .expect("adapt is always offered");
        game.apply(PlayerId::One, action).expect("it activates");
        drain_pending(game);
    };

    activate(&mut game);
    let size = |game: &Game| {
        let krasis = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == krasis_id)
            .expect("still there");
        (game.power(krasis), game.toughness(krasis))
    };
    assert_eq!(size(&game), (Some(7), Some(7)), "three counters arrive");

    activate(&mut game);
    assert_eq!(
        size(&game),
        (Some(7), Some(7)),
        "and a second adapt adds nothing while they are still on it",
    );
}
