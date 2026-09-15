//! Rules-sensitive IKO compositions over existing engine primitives.

use super::*;

fn ready() -> Game {
    let mut game = ready_game();
    game.battlefield.clear();
    for player in &mut game.players {
        player.hand.clear();
        player.graveyard.clear();
    }
    game.turn = 5;
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.priority = PlayerId::One;
    game.step = Step::PrecombatMain;
    game
}

fn settle(game: &mut Game) {
    for _ in 0..40 {
        drain_pending(game);
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            return;
        }
        let player = game.priority;
        game.apply(player, Action::PassPriority).unwrap();
    }
    panic!("resolution did not settle");
}

fn held(game: &mut Game, definition: CardDefinitionId) -> GameObjectId {
    let held = card(980_000, definition, PlayerId::One);
    let id = held.id;
    game.players[0].hand.push(held);
    id
}

#[test]
fn broodmoth_returns_a_stolen_creature_to_its_owner_and_does_not_repeat() {
    let mut game = ready();
    game.put_onto_battlefield(PlayerId::One, cards::LUMINOUS_BROODMOTH)
        .unwrap();
    let mut victim = creature(980_001, cards::GRIZZLY_BEARS, PlayerId::Two);
    victim.controller = PlayerId::One;
    let old = victim.card.id;
    game.battlefield.push(victim);
    game.destroy_permanent(old);
    settle(&mut game);

    let returned = game
        .battlefield
        .iter()
        .find(|p| p.card.definition == cards::GRIZZLY_BEARS)
        .expect("the death trigger returned the creature");
    assert_ne!(returned.card.id, old);
    assert_eq!(returned.controller, PlayerId::Two);
    assert!(game.permanent_has_executable_keyword(returned, KeywordAbility::Flying));
    let returned_id = returned.card.id;
    game.put_onto_battlefield(PlayerId::Two, cards::LUMINOUS_BROODMOTH)
        .unwrap();
    game.destroy_permanent(returned_id);
    settle(&mut game);
    assert!(!game.battlefield.iter().any(|p| p.card.id == returned_id));
    assert!(
        game.players[1]
            .graveyard
            .iter()
            .any(|c| c.definition == cards::GRIZZLY_BEARS)
    );
}

#[test]
fn unbreakable_bond_enters_with_lifelink_and_a_new_object_identity() {
    let mut game = ready();
    let dead = card(980_002, cards::GRIZZLY_BEARS, PlayerId::One);
    let old = dead.id;
    game.players[0].graveyard.push(dead);
    let spell = held(&mut game, cards::UNBREAKABLE_BOND);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 5);
    game.apply(
        PlayerId::One,
        cast_action(spell, vec![Target::Card(old)], vec![], 0),
    )
    .unwrap();
    settle(&mut game);
    let returned = game
        .battlefield
        .iter()
        .find(|p| p.card.definition == cards::GRIZZLY_BEARS)
        .unwrap();
    assert_ne!(returned.card.id, old);
    assert!(game.permanent_has_executable_keyword(returned, KeywordAbility::Lifelink));
    assert_eq!(game.power(returned), Some(2));
}

#[test]
fn dire_tactics_reads_exiled_creatures_last_known_toughness_and_checks_humans() {
    for controls_human in [false, true] {
        let mut game = ready();
        if controls_human {
            game.battlefield
                .push(creature(980_003, cards::CHECKPOINT_OFFICER, PlayerId::One));
        }
        let victim = game
            .put_onto_battlefield(PlayerId::Two, cards::AEGIS_TURTLE)
            .unwrap();
        let spell = held(&mut game, cards::DIRE_TACTICS);
        game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
        game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
        game.apply(
            PlayerId::One,
            cast_action(spell, vec![Target::Permanent(victim)], vec![], 0),
        )
        .unwrap();
        settle(&mut game);
        assert_eq!(game.players[0].life, if controls_human { 20 } else { 15 });
        assert!(
            game.players[1]
                .exile
                .iter()
                .any(|c| c.definition == cards::AEGIS_TURTLE)
        );
    }
}

#[test]
fn magistrate_restricts_only_opponents_and_only_nonhand_casts() {
    let mut game = ready();
    game.put_onto_battlefield(PlayerId::One, cards::DRANNITH_MAGISTRATE)
        .unwrap();
    for (offset, player) in [(0, PlayerId::One), (2, PlayerId::Two)] {
        let index = player.index();
        let grave = card(980_010 + offset, cards::THINK_TWICE, player);
        let hand = card(980_011 + offset, cards::THINK_TWICE, player);
        let grave_id = grave.id;
        let hand_id = hand.id;
        game.players[index].graveyard.push(grave);
        game.players[index].hand.push(hand);
        game.add_unrestricted_mana(player, ManaColor::Blue, 3);
        game.priority = player;
        let actions = game.legal_actions(player);
        let has_cast = |id| {
            actions
                .iter()
                .any(|a| matches!(a, Action::CastSpell { card, .. } if *card == id))
        };
        assert!(has_cast(hand_id));
        assert_eq!(has_cast(grave_id), player == PlayerId::One);
    }
}

#[test]
fn fight_as_one_both_modes_keep_their_separate_targets() {
    let mut game = ready();
    let human = game
        .put_onto_battlefield(PlayerId::One, cards::CHECKPOINT_OFFICER)
        .unwrap();
    let cat = game
        .put_onto_battlefield(PlayerId::One, cards::SAVAI_SABERTOOTH)
        .unwrap();
    let spell = held(&mut game, cards::FIGHT_AS_ONE);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    let choices = CastChoices::default()
        .with_modes(vec![ModeId(0), ModeId(1)])
        .with_targets(vec![
            TargetSelection::new(TargetSlotId(0), vec![Target::Permanent(human)]),
            TargetSelection::new(TargetSlotId(1), vec![Target::Permanent(cat)]),
        ]);
    game.apply(
        PlayerId::One,
        Action::CastSpell {
            card: spell,
            choices,
            sacrifices: vec![],
        },
    )
    .unwrap();
    settle(&mut game);
    for (id, power) in [(human, 2), (cat, 4)] {
        let p = game.battlefield.iter().find(|p| p.card.id == id).unwrap();
        assert_eq!(game.power(p), Some(power));
        assert!(game.permanent_has_executable_keyword(p, KeywordAbility::Indestructible));
    }
}

#[test]
fn narset_selects_damage_targets_only_after_a_nonland_discard() {
    for discarded in [cards::GRIZZLY_BEARS, cards::MOUNTAIN] {
        let mut game = ready();
        game.players[0].library.clear();
        game.players[0]
            .library
            .push(card(980_020, discarded, PlayerId::One));
        let narset = game
            .put_onto_battlefield(PlayerId::One, cards::NARSET_OF_THE_ANCIENT_WAY)
            .unwrap();
        let turtle = game
            .put_onto_battlefield(PlayerId::Two, cards::AEGIS_TURTLE)
            .unwrap();
        let activation = game.legal_actions(PlayerId::One).into_iter().find(|action| {
            matches!(action, Action::ActivateAbility { source, ability, .. }
                if *source == narset && matches!(ability, AbilityOrigin::Printed { ability, .. } if *ability == AbilityId(1)))
        }).expect("Narset can activate -2 without a target");
        game.apply(PlayerId::One, activation).unwrap();
        pass_priority_pair(&mut game);
        let may = game
            .observe(PlayerId::One)
            .decision
            .expect("the discard is optional");
        let accept = may
            .options
            .iter()
            .find(|option| option.label != "Decline")
            .unwrap()
            .id;
        game.apply(
            PlayerId::One,
            Action::ChooseDecision {
                decision: may.id,
                options: vec![accept],
            },
        )
        .unwrap();
        assert!(game.players[0].hand.is_empty());
        assert!(
            game.players[0]
                .graveyard
                .iter()
                .any(|card| card.definition == discarded)
        );
        if discarded == cards::MOUNTAIN {
            assert!(game.stack.is_empty());
            assert!(game.pending_decisions.is_empty());
        } else {
            let target = game
                .observe(PlayerId::One)
                .decision
                .expect("a nonland discard creates a fresh target choice");
            let option = target
                .options
                .iter()
                .find(|option| option.card.is_some_and(|(id, _)| id == turtle))
                .unwrap()
                .id;
            game.apply(
                PlayerId::One,
                Action::ChooseDecision {
                    decision: target.id,
                    options: vec![option],
                },
            )
            .unwrap();
            assert_eq!(
                game.stack.len(),
                1,
                "the reflexive trigger can be responded to"
            );
            pass_priority_pair(&mut game);
        }
        let turtle = game
            .battlefield
            .iter()
            .find(|p| p.card.id == turtle)
            .unwrap();
        assert_eq!(
            turtle.damage,
            if discarded == cards::MOUNTAIN { 0 } else { 2 }
        );
    }
}

#[test]
fn back_for_more_targets_the_fight_only_after_a_successful_arrival() {
    for redirected in [false, true] {
        let mut game = ready();
        if redirected {
            game.battlefield
                .push(creature(980_030, cards::CONTAINMENT_PRIEST, PlayerId::Two));
        }
        let turtle = game
            .put_onto_battlefield(PlayerId::Two, cards::AEGIS_TURTLE)
            .unwrap();
        let body = card(980_031, cards::GRIZZLY_BEARS, PlayerId::One);
        let body_id = body.id;
        game.players[0].graveyard.push(body);
        let spell = held(&mut game, cards::BACK_FOR_MORE);
        game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 5);
        game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 1);
        game.apply(
            PlayerId::One,
            cast_action(spell, vec![Target::Card(body_id)], vec![], 0),
        )
        .unwrap();
        pass_priority_pair(&mut game);
        if redirected {
            assert!(game.pending_decisions.is_empty());
            assert!(game.stack.is_empty());
            assert!(
                game.players[0]
                    .exile
                    .iter()
                    .any(|c| c.definition == cards::GRIZZLY_BEARS)
            );
        } else {
            let decision = game
                .observe(PlayerId::One)
                .decision
                .expect("arrival creates the fight target choice");
            let option = decision
                .options
                .iter()
                .find(|option| option.card.is_some_and(|(id, _)| id == turtle))
                .unwrap()
                .id;
            game.apply(
                PlayerId::One,
                Action::ChooseDecision {
                    decision: decision.id,
                    options: vec![option],
                },
            )
            .unwrap();
            assert_eq!(game.stack.len(), 1);
            assert!(
                game.battlefield
                    .iter()
                    .any(|p| p.card.definition == cards::GRIZZLY_BEARS && p.card.id != body_id)
            );
            pass_priority_pair(&mut game);
        }
        assert_eq!(
            game.battlefield
                .iter()
                .find(|p| p.card.id == turtle)
                .unwrap()
                .damage,
            if redirected { 0 } else { 2 }
        );
    }
}

#[test]
fn of_one_mind_requires_both_a_human_and_a_nonhuman_for_its_discount() {
    for occupants in 0..4 {
        let mut game = ready();
        if occupants & 1 != 0 {
            game.put_onto_battlefield(PlayerId::One, cards::CHECKPOINT_OFFICER)
                .unwrap();
        }
        if occupants & 2 != 0 {
            game.put_onto_battlefield(PlayerId::One, cards::AEGIS_TURTLE)
                .unwrap();
        }
        let spell = held(&mut game, cards::OF_ONE_MIND);
        game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
        let action = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .find(|a| matches!(a, Action::CastSpell { card, .. } if *card == spell));
        assert_eq!(action.is_some(), occupants == 3);
        if let Some(action) = action {
            game.apply(PlayerId::One, action).unwrap();
        }
    }
}

#[test]
fn cunning_nightbonder_prices_and_protects_only_spells_with_flash() {
    for flash in [false, true] {
        let mut game = ready();
        let bonder = game
            .put_onto_battlefield(PlayerId::One, cards::CUNNING_NIGHTBONDER)
            .unwrap();
        let definition = if flash {
            cards::CRYSTACEAN
        } else {
            cards::GRIZZLY_BEARS
        };
        let spell = held(&mut game, definition);
        game.add_unrestricted_mana(
            PlayerId::One,
            if flash {
                ManaColor::Blue
            } else {
                ManaColor::Green
            },
            if flash { 3 } else { 2 },
        );
        game.apply(PlayerId::One, cast_action(spell, vec![], vec![], 0))
            .unwrap();
        assert_eq!(game.can_be_countered(game.stack.last().unwrap()), !flash);
        game.destroy_permanent(bonder);
        assert!(game.can_be_countered(game.stack.last().unwrap()));
    }
}

#[test]
fn offspring_copies_the_exiled_successor_with_one_one_stats_and_haste() {
    let mut game = ready();
    game.players[0]
        .graveyard
        .push(card(980_040, cards::CHECKPOINT_OFFICER, PlayerId::One));
    game.put_onto_battlefield(PlayerId::One, cards::OFFSPRING_S_REVENGE)
        .unwrap();
    game.advance_step();
    game.finish_rules_procedure();
    settle(&mut game);
    assert!(game.players[0].graveyard.is_empty());
    assert!(
        game.players[0]
            .exile
            .iter()
            .any(|card| card.definition == cards::CHECKPOINT_OFFICER)
    );
    let token = game
        .battlefield
        .iter()
        .find(|p| p.card.definition.is_token())
        .expect("a copy was created from exile");
    assert_eq!(
        (game.power(token), game.toughness(token)),
        (Some(1), Some(1))
    );
    assert!(game.permanent_has_executable_keyword(token, KeywordAbility::Haste));
    assert_eq!(game.object_subtypes(token.card.id), &["Human", "Soldier"]);
}

#[test]
fn fire_prophecy_keeps_the_hand_choice_private_and_draws_after_bottoming() {
    let mut game = ready();
    game.players[0].library.clear();
    game.players[0]
        .library
        .push(card(980_050, cards::FOREST, PlayerId::One));
    game.players[0]
        .hand
        .push(card(980_051, cards::MOUNTAIN, PlayerId::One));
    game.players[0]
        .hand
        .push(card(980_052, cards::GRIZZLY_BEARS, PlayerId::One));
    let turtle = game
        .put_onto_battlefield(PlayerId::Two, cards::AEGIS_TURTLE)
        .unwrap();
    let spell = held(&mut game, cards::FIRE_PROPHECY);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 2);
    game.apply(
        PlayerId::One,
        cast_action(spell, vec![Target::Permanent(turtle)], vec![], 0),
    )
    .unwrap();
    pass_priority_pair(&mut game);
    let choice = game
        .observe(PlayerId::One)
        .decision
        .expect("optional hand choice");
    assert_eq!(choice.visibility, DecisionVisibility::Private);
    assert!(!choice.options_visible_to(PlayerId::Two));
    choose_decision_by_label(&mut game, PlayerId::One, "Mountain");
    assert_eq!(game.players[0].library[0].definition, cards::MOUNTAIN);
    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::FOREST)
    );
    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS)
    );
}
