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

#[test]
fn the_soft_counters_ask_for_their_own_amount() {
    for (definition, color, tax) in [
        (cards::MANA_TITHE, ManaColor::White, 1),
        (cards::SPELL_PIERCE, ManaColor::Blue, 2),
        (cards::MISCALCULATION, ManaColor::Blue, 2),
    ] {
        let mut game = ready_game();
        let bolt = card(50_800, cards::LIGHTNING_BOLT, PlayerId::Two);
        let bolt_id = bolt.id;
        game.players[PlayerId::Two.index()].hand.push(bolt);
        game.players[PlayerId::Two.index()].mana_pool.red = 1;
        // Enough left over to pay the tax, so the choice is a real one.
        game.players[PlayerId::Two.index()].mana_pool.colorless = tax;
        game.priority = PlayerId::Two;
        game.apply(
            PlayerId::Two,
            cast_action(bolt_id, vec![Target::Player(PlayerId::One)], Vec::new(), 0),
        )
        .expect("the opponent casts something to counter");
        let on_stack = game.stack.last().expect("the spell is on the stack").id;
        game.apply(PlayerId::Two, Action::PassPriority).unwrap();

        let counter = card(50_801, definition, PlayerId::One);
        let counter_id = counter.id;
        game.players[PlayerId::One.index()].hand.push(counter);
        let pool = &mut game.players[PlayerId::One.index()].mana_pool;
        pool.add_color(color, 1);
        pool.colorless += 1;
        game.apply(
            PlayerId::One,
            cast_action(counter_id, vec![Target::Spell(on_stack)], Vec::new(), 0),
        )
        .unwrap_or_else(|error| panic!("{definition:?} answers a spell: {error}"));
        pass_priority_pair(&mut game);

        // Declining the tax is what counters the spell.
        let decision = game
            .observe(PlayerId::Two)
            .decision
            .unwrap_or_else(|| panic!("{definition:?} asks its controller to pay {tax}"));
        let decline = decision
            .options
            .iter()
            .find(|option| option.label != "Pay the cost")
            .unwrap_or_else(|| panic!("{definition:?} offers declining"))
            .id;
        game.apply(
            PlayerId::Two,
            Action::ChooseDecision {
                decision: decision.id,
                options: vec![decline],
            },
        )
        .expect("declining is allowed");
        assert!(
            game.players[PlayerId::Two.index()]
                .graveyard
                .iter()
                .any(|card| card.definition == cards::LIGHTNING_BOLT),
            "{definition:?} counters what went unpaid",
        );
    }
}

#[test]
fn the_monolith_makes_three_and_stays_tapped_until_it_is_bought_back() {
    let mut game = ready_game();
    let monolith = game
        .put_onto_battlefield(PlayerId::One, cards::GRIM_MONOLITH)
        .expect("cataloged");
    game.apply(
        PlayerId::One,
        Action::ActivateManaAbility {
            source: monolith,
            ability: mana_ability_for(&game, monolith, ManaColor::Colorless),
            color: ManaColor::Colorless,
            counters_removed: None,
            cost_object: None,
        },
    )
    .expect("it taps for mana");
    assert_eq!(game.players[PlayerId::One.index()].mana_pool.colorless, 3);

    // Four of that mana buys the untap back; three does not.
    game.players[PlayerId::One.index()].mana_pool.colorless = 3;
    assert!(
        activation(&game, monolith).is_none(),
        "three mana does not pay the untap",
    );
    game.players[PlayerId::One.index()].mana_pool.colorless = 4;
    let untap = activation(&game, monolith).expect("four mana pays it");
    game.apply(PlayerId::One, untap).expect("it is activated");
    resolve(&mut game);
    assert!(
        !game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == monolith)
            .expect("still on the battlefield")
            .tapped,
    );
}

#[test]
fn the_mind_stone_trades_itself_for_a_card() {
    let mut game = ready_game();
    let stone = game
        .put_onto_battlefield(PlayerId::One, cards::MIND_STONE)
        .expect("cataloged");
    game.players[PlayerId::One.index()].mana_pool.colorless = 1;
    let before = game.players[PlayerId::One.index()].library.len();

    let action = activation(&game, stone).expect("the draw ability is offered");
    game.apply(PlayerId::One, action).expect("it is activated");
    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != stone),
        "the sacrifice is a cost",
    );
    resolve(&mut game);
    assert_eq!(
        game.players[PlayerId::One.index()].library.len(),
        before - 1
    );
}

#[test]
fn mother_of_runes_protects_a_creature_from_the_color_she_names() {
    let mut game = ready_game();
    game.battlefield.clear();
    // She has been here since before this turn, so her tap ability is live.
    let mother_permanent = creature(50_899, cards::MOTHER_OF_RUNES, PlayerId::One);
    let mother = mother_permanent.card.id;
    game.battlefield.push(mother_permanent);
    let bears = creature(50_900, cards::GRIZZLY_BEARS, PlayerId::One);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { source, targets, .. }
            if *source == mother
                && targets.iter().any(|selection| {
                    selection.targets().contains(&Target::Permanent(bears_id))
                }))
        })
        .expect("she can target another creature you control");
    game.apply(PlayerId::One, action).expect("it is activated");
    pass_priority_pair(&mut game);

    let decision = game
        .observe(PlayerId::One)
        .decision
        .expect("the color is chosen on resolution");
    let red = decision
        .options
        .iter()
        .find(|option| option.label.eq_ignore_ascii_case("red"))
        .expect("red is one of the colors")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![red],
        },
    )
    .expect("the color is chosen");

    // A red spell can no longer touch what she named.
    let bolt = card(50_901, cards::LIGHTNING_BOLT, PlayerId::Two);
    let bolt_id = bolt.id;
    game.players[PlayerId::Two.index()].hand.push(bolt);
    game.players[PlayerId::Two.index()].mana_pool.red = 1;
    game.priority = PlayerId::Two;
    assert!(
        !game.legal_actions(PlayerId::Two).iter().any(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
            if *card == bolt_id
                && choices.targets().iter().any(|selection| {
                    selection.targets().contains(&Target::Permanent(bears_id))
                }))
        }),
        "the protected creature is not an offered target",
    );
}

/// Equips `source` to `host`, then settles whatever that started.
fn equip_to(game: &mut Game, source: GameObjectId, host: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source: actual,
                targets,
                ..
            } => {
                *actual == source
                    && targets
                        .iter()
                        .flat_map(crate::casting::TargetSelection::targets)
                        .any(|target| *target == Target::Permanent(host))
            }
            _ => false,
        })
        .expect("equip is offered for that creature");
    game.apply(PlayerId::One, action)
        .expect("the ability activates");
    drain_pending(game);
}

/// The Clamp on something that survives it: bigger, frailer, still there.
#[test]
fn the_clamp_gives_plus_one_minus_one_to_what_it_equips() {
    let mut game = ready_game();
    game.battlefield.clear();
    let clamp = creature(51_000, cards::SKULLCLAMP, PlayerId::One);
    let clamp_id = clamp.card.id;
    game.battlefield.push(clamp);
    let bears = creature(51_001, cards::GRIZZLY_BEARS, PlayerId::One);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    game.players[PlayerId::One.index()].mana_pool.colorless = 1;
    let before = game.players[PlayerId::One.index()].library.len();

    equip_to(&mut game, clamp_id, bears_id);

    let bears = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == bears_id)
        .expect("a 2/2 survives losing a toughness");
    assert_eq!(
        (game.power(bears), game.toughness(bears)),
        (Some(3), Some(1))
    );
    assert_eq!(
        game.players[PlayerId::One.index()].library.len(),
        before,
        "nothing is drawn while the creature is alive",
    );
}

/// The Clamp on a one-toughness creature, which is the whole point of it: the
/// minus kills what it equips through state-based actions, and the trigger
/// still finds the creature it was attached to a moment ago.
#[test]
fn the_clamp_kills_a_one_toughness_creature_and_draws_two() {
    let mut game = ready_game();
    game.battlefield.clear();
    let clamp = creature(51_100, cards::SKULLCLAMP, PlayerId::One);
    let clamp_id = clamp.card.id;
    game.battlefield.push(clamp);
    let lions = creature(51_101, cards::SAVANNAH_LIONS, PlayerId::One);
    let lions_id = lions.card.id;
    game.battlefield.push(lions);
    game.players[PlayerId::One.index()].mana_pool.colorless = 1;
    let before = game.players[PlayerId::One.index()].library.len();

    equip_to(&mut game, clamp_id, lions_id);

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != lions_id),
        "a 2/1 loses its last toughness and dies",
    );
    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::SAVANNAH_LIONS),
    );
    assert_eq!(
        game.players[PlayerId::One.index()].library.len(),
        before - 2,
        "the Clamp draws two off a creature that was gone before it triggered",
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == clamp_id),
        "the Clamp stays behind, ready for the next one",
    );
}

/// Life: the lands stay lands, which is the half of the sentence that matters.
#[test]
fn life_animates_your_lands_without_taking_their_land_type_away() {
    let mut game = ready_game();
    game.battlefield.clear();
    let forest = creature(53_000, cards::FOREST, PlayerId::One);
    let forest_id = forest.card.id;
    game.battlefield.push(forest);
    // Someone else's land is not "you control".
    let island = creature(53_001, cards::ISLAND, PlayerId::Two);
    let island_id = island.card.id;
    game.battlefield.push(island);

    let life = card(53_002, cards::LIFE_DEATH, PlayerId::One);
    let life_id = life.id;
    game.players[PlayerId::One.index()].hand.push(life);
    game.players[PlayerId::One.index()].mana_pool.green = 1;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if *card == life_id && choices.play_option() == PlayOptionId::DEFAULT)
        })
        .expect("Life is the first half");
    game.apply(PlayerId::One, action).expect("it is cast");
    drain_pending(&mut game);

    let types = |id| {
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == id)
            .and_then(|permanent| game.permanent_types(permanent))
            .expect("the land is still there")
    };
    assert!(types(forest_id).contains(crate::card::CardType::Creature));
    assert!(
        types(forest_id).contains(crate::card::CardType::Land),
        "they're still lands",
    );
    let forest = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == forest_id)
        .expect("the Forest is still there");
    assert_eq!(
        (game.power(forest), game.toughness(forest)),
        (Some(1), Some(1)),
    );
    assert!(
        !types(island_id).contains(crate::card::CardType::Creature),
        "only lands you control are animated",
    );
}

/// Death: the other half of the same card, reaching only into your own
/// graveyard and charging you the creature's mana value in life.
#[test]
fn death_reanimates_from_your_own_graveyard_for_its_mana_value_in_life() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].graveyard.push(card(
        53_100,
        cards::SERRA_ANGEL,
        PlayerId::One,
    ));
    game.players[PlayerId::Two.index()].graveyard.push(card(
        53_101,
        cards::GRIZZLY_BEARS,
        PlayerId::Two,
    ));

    let death = card(53_102, cards::LIFE_DEATH, PlayerId::One);
    let death_id = death.id;
    game.players[PlayerId::One.index()].hand.push(death);
    game.players[PlayerId::One.index()].mana_pool.black = 1;
    game.players[PlayerId::One.index()].mana_pool.colorless = 1;
    let life = game.players[PlayerId::One.index()].life;

    let offered = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell { card, choices, .. }
                if card == death_id && choices.play_option() == PlayOptionId(1) =>
            {
                Some(choices.targets().to_vec())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        offered.len(),
        1,
        "only the creature in your own graveyard is a legal target",
    );

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if *card == death_id && choices.play_option() == PlayOptionId(1))
        })
        .expect("Death is the second half");
    game.apply(PlayerId::One, action).expect("it is cast");
    drain_pending(&mut game);

    assert!(
        game.battlefield.iter().any(|permanent| {
            permanent.controller == PlayerId::One && permanent.card.definition == cards::SERRA_ANGEL
        }),
        "the angel comes back under your control",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].life,
        life - 5,
        "a five-drop costs five life",
    );
}

/// Rancor on a creature: bigger, trampling, and still there.
#[test]
fn rancor_grants_two_power_and_trample() {
    let mut game = ready_game();
    game.battlefield.clear();
    let bears = creature(54_000, cards::GRIZZLY_BEARS, PlayerId::One);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    let rancor = card(54_001, cards::RANCOR, PlayerId::One);
    let rancor_id = rancor.id;
    game.players[PlayerId::One.index()].hand.push(rancor);
    game.players[PlayerId::One.index()].mana_pool.green = 1;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
            if *card == rancor_id
                && choices.targets().iter().any(|selection| {
                    selection.targets().contains(&Target::Permanent(bears_id))
                }))
        })
        .expect("Rancor targets a creature");
    game.apply(PlayerId::One, action).expect("it is cast");
    drain_pending(&mut game);

    let bears = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == bears_id)
        .expect("the creature is still there");
    assert_eq!(
        (game.power(bears), game.toughness(bears)),
        (Some(4), Some(2))
    );
    assert!(game.has_trample(bears));
}

/// The clause the card is remembered for. Whichever half of the pair is
/// answered, the Aura reaches the graveyard, and it is the graveyard object
/// -- a different object from the permanent that just left -- that comes back
/// to hand.
#[test]
fn rancor_returns_itself_to_hand_from_the_graveyard() {
    for kill_the_creature in [false, true] {
        let mut game = ready_game();
        game.battlefield.clear();
        game.players[PlayerId::One.index()].hand.clear();
        let bears = creature(54_100, cards::GRIZZLY_BEARS, PlayerId::One);
        let bears_id = bears.card.id;
        game.battlefield.push(bears);
        let rancor = card(54_101, cards::RANCOR, PlayerId::One);
        let rancor_id = rancor.id;
        game.players[PlayerId::One.index()].hand.push(rancor);
        game.players[PlayerId::One.index()].mana_pool.green = 1;

        let action = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| {
                matches!(action, Action::CastSpell { card, choices, .. }
                if *card == rancor_id
                    && choices.targets().iter().any(|selection| {
                        selection.targets().contains(&Target::Permanent(bears_id))
                    }))
            })
            .expect("Rancor targets a creature");
        game.apply(PlayerId::One, action).expect("it is cast");
        drain_pending(&mut game);

        // The Aura on the battlefield is a new object; the hand card's id is
        // not the permanent's.
        let aura = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.definition == cards::RANCOR)
            .expect("the Aura attached")
            .card
            .id;
        let doomed = if kill_the_creature { bears_id } else { aura };
        game.move_permanents_to_graveyard(&[doomed]);
        // Killing the creature leaves the Aura enchanting nothing; it takes
        // state-based actions to notice and send it after its host.
        game.check_state_based_actions();
        drain_pending(&mut game);

        assert!(
            game.players[PlayerId::One.index()]
                .hand
                .iter()
                .any(|card| card.definition == cards::RANCOR),
            "Rancor comes back whether the creature or the Aura was answered \
             (creature killed: {kill_the_creature})",
        );
        assert!(
            game.players[PlayerId::One.index()]
                .graveyard
                .iter()
                .all(|card| card.definition != cards::RANCOR),
            "and it does not stay in the graveyard as well",
        );
    }
}
