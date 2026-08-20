//! Spells and permanents cataloged for the Vintage Cube pool.

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

/// "Power or toughness 2 or less" is a disjunction. A 4/1 qualifies on
/// toughness and a 2/3 on power; only a creature big in both directions is
/// safe, and a noncreature spell was never in question.
#[test]
fn stern_scolding_answers_a_spell_small_in_either_direction() {
    for (spell, counterable) in [
        (cards::GRIZZLY_BEARS, true),
        // A 4/1: too big to be caught by power, small enough by toughness.
        (cards::PHANTASMAL_FORCES, true),
        // A 2/3: the mirror of it.
        (cards::ERG_RAIDERS, true),
        (cards::SERRA_ANGEL, false),
        // Not a creature spell at all.
        (cards::LIGHTNING_BOLT, false),
    ] {
        // The active player casts the creature; the other one answers it.
        let mut game = ready_game();
        let cast = card(55_000, spell, PlayerId::One);
        let cast_id = cast.id;
        game.players[PlayerId::One.index()].hand.push(cast);
        let pool = &mut game.players[PlayerId::One.index()].mana_pool;
        pool.white = 5;
        pool.blue = 5;
        pool.black = 5;
        pool.red = 5;
        pool.green = 5;
        pool.colorless = 5;
        let cast_action = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == cast_id))
            .unwrap_or_else(|| panic!("{spell:?} is castable"));
        game.apply(PlayerId::One, cast_action).expect("it is cast");
        let on_stack = game.stack.last().expect("it is on the stack").id;
        game.apply(PlayerId::One, Action::PassPriority).unwrap();

        let scolding = card(55_001, cards::STERN_SCOLDING, PlayerId::Two);
        let scolding_id = scolding.id;
        game.players[PlayerId::Two.index()].hand.push(scolding);
        game.players[PlayerId::Two.index()].mana_pool.blue = 1;

        let offered = |game: &Game| {
            game.legal_actions(PlayerId::Two)
                .into_iter()
                .find(|action| {
                    matches!(action, Action::CastSpell { card, choices, .. }
                    if *card == scolding_id
                        && choices.targets().iter().any(|selection| {
                            selection.targets().contains(&Target::Spell(on_stack))
                        }))
                })
        };
        let action = offered(&game);
        assert_eq!(
            action.is_some(),
            counterable,
            "{spell:?} should{} be a legal target",
            if counterable { "" } else { " not" },
        );

        let Some(action) = action else {
            continue;
        };
        game.apply(PlayerId::Two, action).expect("it is cast");
        drain_pending(&mut game);
        assert!(
            game.players[PlayerId::One.index()]
                .graveyard
                .iter()
                .any(|card| card.definition == spell),
            "{spell:?} is countered",
        );
        assert!(
            game.battlefield
                .iter()
                .all(|permanent| permanent.card.definition != spell),
            "{spell:?} never reaches the battlefield",
        );
    }
}

/// The free cast is gated on whose turn it is, and the printed one is not.
/// A green card in hand pays for it only while someone else is the active
/// player.
#[test]
fn force_of_vigor_is_free_only_on_someone_elses_turn() {
    let free_cast_offered = |active: PlayerId| {
        let mut game = ready_game();
        game.active_player = active;
        let force = card(57_000, cards::FORCE_OF_VIGOR, PlayerId::One);
        let force_id = force.id;
        game.players[PlayerId::One.index()].hand.push(force);
        game.players[PlayerId::One.index()].hand.push(card(
            57_001,
            cards::BIRDS_OF_PARADISE,
            PlayerId::One,
        ));
        game.battlefield
            .push(creature(57_002, cards::BLACK_LOTUS, PlayerId::Two));
        game.legal_actions(PlayerId::One).into_iter().any(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if card == force_id && choices.costs().alternative().is_some())
        })
    };

    assert!(
        !free_cast_offered(PlayerId::One),
        "on your own turn there is no free cast, whatever is in hand",
    );
    assert!(
        free_cast_offered(PlayerId::Two),
        "on someone else's turn a green card pays for it",
    );
}

/// "Up to two" means the spell can take one, and "artifacts and/or
/// enchantments" means it does not care which.
#[test]
fn force_of_vigor_destroys_both_kinds_at_once() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.active_player = PlayerId::Two;
    let lotus = creature(57_100, cards::BLACK_LOTUS, PlayerId::Two);
    let lotus_id = lotus.card.id;
    game.battlefield.push(lotus);
    let arena = creature(57_101, cards::PHYREXIAN_ARENA, PlayerId::Two);
    let arena_id = arena.card.id;
    game.battlefield.push(arena);
    // Not an artifact or an enchantment, so never a legal target.
    let bears = creature(57_102, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);

    let force = card(57_103, cards::FORCE_OF_VIGOR, PlayerId::One);
    let force_id = force.id;
    game.players[PlayerId::One.index()].hand.push(force);
    game.players[PlayerId::One.index()].hand.push(card(
        57_104,
        cards::BIRDS_OF_PARADISE,
        PlayerId::One,
    ));

    assert!(
        !game.legal_actions(PlayerId::One).iter().any(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
            if *card == force_id
                && choices.targets().iter().any(|selection| {
                    selection.targets().contains(&Target::Permanent(bears_id))
                }))
        }),
        "a creature is not an artifact or an enchantment",
    );

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
            if *card == force_id
                && choices.costs().alternative().is_some()
                && choices.targets().iter().any(|selection| {
                    selection.targets().contains(&Target::Permanent(lotus_id))
                        && selection.targets().contains(&Target::Permanent(arena_id))
                }))
        })
        .expect("both halves of the board can go at once");
    game.apply(PlayerId::One, action).expect("it is cast");
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != lotus_id && permanent.card.id != arena_id),
        "the artifact and the enchantment are both destroyed",
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == bears_id),
        "and the creature is untouched",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].exile.len(),
        1,
        "the green card it spent was exiled, not discarded",
    );
}

/// Two damage, and then the same two again from the graveyard -- after which
/// the card is exiled rather than left to be flashed back twice.
#[test]
fn firebolt_burns_from_hand_and_once_more_from_the_graveyard() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    let bolt = card(73_000, cards::FIREBOLT, PlayerId::One);
    let bolt_id = bolt.id;
    game.players[0].hand.push(bolt);
    game.players[0].mana_pool.red = 1;
    let start = game.players[PlayerId::Two.index()].life;

    game.apply(
        PlayerId::One,
        cast_action(bolt_id, vec![Target::Player(PlayerId::Two)], Vec::new(), 0),
    )
    .expect("it is cast from hand");
    drain_pending(&mut game);
    assert_eq!(game.players[PlayerId::Two.index()].life, start - 2);
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::FIREBOLT),
        "and it lands in the graveyard, where the flashback lives",
    );

    let from_graveyard = game.players[0]
        .graveyard
        .iter()
        .find(|card| card.definition == cards::FIREBOLT)
        .expect("still there")
        .id;
    game.players[0].mana_pool.red = 1;
    game.players[0].mana_pool.colorless = 4;
    let flashback = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
            if *card == from_graveyard
                && choices.targets().iter().any(|selection| {
                    selection.targets().contains(&Target::Player(PlayerId::Two))
                }))
        })
        .expect("flashback is offered from the graveyard");
    game.apply(PlayerId::One, flashback).expect("it is cast");
    drain_pending(&mut game);

    assert_eq!(game.players[PlayerId::Two.index()].life, start - 4);
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .all(|card| card.definition != cards::FIREBOLT),
        "flashback exiles it rather than returning it",
    );
    assert_eq!(game.players[0].exile.len(), 1);
}

/// The chain is the opponent's to continue. Unlike Chain of Vapor, passing it
/// on costs nothing -- so what stops it is a player choosing to stop it, or
/// running out of cards to lose.
#[test]
fn chain_of_smog_discards_two_and_offers_the_chain_back() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::Two.index()].hand.clear();
    for (instance, definition) in [
        (75_000, cards::LIGHTNING_BOLT),
        (75_001, cards::SERRA_ANGEL),
        (75_002, cards::FOREST),
    ] {
        game.players[PlayerId::Two.index()]
            .hand
            .push(card(instance, definition, PlayerId::Two));
    }

    let chain = card(75_100, cards::CHAIN_OF_SMOG, PlayerId::One);
    let chain_id = chain.id;
    game.players[PlayerId::One.index()].hand.push(chain);
    game.players[PlayerId::One.index()].mana_pool.black = 1;
    game.players[PlayerId::One.index()].mana_pool.colorless = 1;
    game.priority = PlayerId::One;
    game.apply(
        PlayerId::One,
        cast_action(chain_id, vec![Target::Player(PlayerId::Two)], Vec::new(), 0),
    )
    .expect("it can name a player");

    // The discard is theirs to choose, so answer it for them.
    for _ in 0..8 {
        let Some(decision) = game.observe(PlayerId::Two).decision else {
            let player = game.priority;
            if game.apply(player, Action::PassPriority).is_err() {
                break;
            }
            continue;
        };
        if decision.prompt.contains("copy") {
            // The chain offer: this is what the test came for.
            assert_eq!(
                game.players[PlayerId::Two.index()].hand.len(),
                1,
                "two cards went first",
            );
            assert_eq!(game.players[PlayerId::Two.index()].graveyard.len(), 2);
            return;
        }
        let chosen = decision
            .options
            .iter()
            .take(decision.minimum)
            .map(|option| option.id)
            .collect::<Vec<_>>();
        game.apply(
            PlayerId::Two,
            Action::ChooseDecision {
                decision: decision.id,
                options: chosen,
            },
        )
        .expect("the discard choice is legal");
    }
    panic!("the chain was never offered back to the player who was hit");
}

/// The free cast is gated twice: on a Swamp, and on having the life to
/// spend. Both are checked before the option is offered rather than at
/// resolution, so an unpayable alternative never appears as a legal action.
#[test]
fn snuff_out_is_free_only_with_a_swamp_and_the_life_to_pay() {
    let free_offered = |swamp: bool, life: i16| {
        let mut game = ready_game();
        game.battlefield.clear();
        if swamp {
            game.battlefield
                .push(creature(79_000, cards::SWAMP, PlayerId::One));
        }
        game.players[PlayerId::One.index()].life = life;
        let snuff = card(79_001, cards::SNUFF_OUT, PlayerId::One);
        let snuff_id = snuff.id;
        game.players[PlayerId::One.index()].hand.push(snuff);
        game.battlefield
            .push(creature(79_002, cards::GRIZZLY_BEARS, PlayerId::Two));
        game.legal_actions(PlayerId::One).into_iter().any(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if card == snuff_id && choices.costs().alternative().is_some())
        })
    };

    assert!(
        free_offered(true, 20),
        "a Swamp and twenty life pays for it"
    );
    assert!(!free_offered(false, 20), "no Swamp, no free cast");
    // CR 118.4: life may be paid when the total is at least the amount, so
    // exactly four is payable and takes its controller to zero.
    assert!(free_offered(true, 4), "four life can pay four");
    assert!(
        !free_offered(true, 3),
        "and three cannot, so the option is not offered at all",
    );
}

/// Casting it for free costs the four life and kills what it names -- and it
/// will not name a black creature.
#[test]
fn snuff_out_pays_four_life_and_destroys_a_nonblack_creature() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.battlefield
        .push(creature(79_100, cards::SWAMP, PlayerId::One));
    let bears = creature(79_101, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    let djinn = creature(79_102, cards::JUZAM_DJINN, PlayerId::Two);
    let djinn_id = djinn.card.id;
    game.battlefield.push(djinn);
    game.players[PlayerId::One.index()].life = 20;

    let snuff = card(79_103, cards::SNUFF_OUT, PlayerId::One);
    let snuff_id = snuff.id;
    game.players[PlayerId::One.index()].hand.push(snuff);

    assert!(
        !game.legal_actions(PlayerId::One).iter().any(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
            if *card == snuff_id
                && choices.targets().iter().any(|selection| {
                    selection.targets().contains(&Target::Permanent(djinn_id))
                }))
        }),
        "a black creature is not a legal target",
    );

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
            if *card == snuff_id
                && choices.costs().alternative().is_some()
                && choices.targets().iter().any(|selection| {
                    selection.targets().contains(&Target::Permanent(bears_id))
                }))
        })
        .expect("the free cast can name the green creature");
    game.apply(PlayerId::One, cast).expect("it is cast");
    assert_eq!(
        game.players[PlayerId::One.index()].life,
        16,
        "the life is paid as the spell is cast",
    );
    drain_pending(&mut game);
    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != bears_id),
        "and the creature is destroyed",
    );
}

/// Delirium counts card types in your own graveyard, not both. Four types is
/// the line: below it the spell deals two, at it six.
#[test]
fn unholy_heat_deals_six_only_with_four_card_types_in_your_graveyard() {
    for (types, expected) in [(3_usize, 2_i16), (4, 6)] {
        let mut game = ready_game();
        game.battlefield.clear();
        game.players[PlayerId::One.index()].graveyard.clear();
        game.players[PlayerId::Two.index()].graveyard.clear();
        // A fifth type sitting in the opponent's graveyard must not count.
        game.players[PlayerId::Two.index()].graveyard.push(card(
            80_000,
            cards::BLACK_LOTUS,
            PlayerId::Two,
        ));

        for (index, definition) in [
            cards::GRIZZLY_BEARS,
            cards::LIGHTNING_BOLT,
            cards::FOREST,
            cards::PHYREXIAN_ARENA,
        ]
        .into_iter()
        .take(types)
        .enumerate()
        {
            game.players[PlayerId::One.index()].graveyard.push(card(
                80_100 + u32::try_from(index).expect("four cards fit"),
                definition,
                PlayerId::One,
            ));
        }

        let angel = creature(80_200, cards::SERRA_ANGEL, PlayerId::Two);
        let angel_id = angel.card.id;
        game.battlefield.push(angel);

        let heat = card(80_201, cards::UNHOLY_HEAT, PlayerId::One);
        let heat_id = heat.id;
        game.players[PlayerId::One.index()].hand.push(heat);
        game.players[PlayerId::One.index()].mana_pool.red = 1;
        game.apply(
            PlayerId::One,
            cast_action(heat_id, vec![Target::Permanent(angel_id)], Vec::new(), 0),
        )
        .expect("it can name a creature");
        drain_pending(&mut game);

        let dealt = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == angel_id)
            .map_or(6, |permanent| {
                i16::try_from(permanent.damage).unwrap_or(i16::MAX)
            });
        assert_eq!(
            dealt, expected,
            "{types} card types in your graveyard, and an artifact in theirs",
        );
    }
}
