use super::*;

fn setup() -> (Game, GameObjectId) {
    let mut game = board(&[]);
    let source = game
        .put_onto_battlefield(PlayerId::One, cards::MOONSHADOW)
        .unwrap();
    (game, source)
}

fn counters(game: &Game) -> u16 {
    permanent(game, cards::MOONSHADOW).counters(CounterKind::MinusOneMinusOne)
}

fn triggers(game: &Game, source: GameObjectId) -> usize {
    game.pending_triggers
        .iter()
        .filter(|trigger| trigger.source.object == source)
        .count()
}

fn mill(game: &mut Game, player: PlayerId, cards: &[CardDefinitionId]) {
    game.set_library(player, cards).unwrap();
    let source = spell(100_000, cards::TOME_SCOUR, player, 0);
    game.resolve_effect_def(
        ScopedEffect::primary(EffectDef::Mill {
            player: EffectRecipientDef::Controller,
            amount: ValueDef::Constant(cards.len().try_into().unwrap()),
        }),
        &source,
        EffectResolutionContext::empty(),
    );
}

#[test]
fn moonshadow_cast_enters_as_one_one_with_menace() {
    let mut game = board(&[]);
    cast(&mut game, cards::MOONSHADOW);
    let moonshadow = permanent(&game, cards::MOONSHADOW);
    assert_eq!(counters(&game), 6);
    assert_eq!(game.power(moonshadow), Some(1));
    assert_eq!(game.toughness(moonshadow), Some(1));
    assert!(game.permanent_has_executable_keyword(moonshadow, KeywordAbility::Menace));
}

#[test]
fn moonshadow_mill_groups_permanent_cards_but_keeps_separate_instructions() {
    let (mut game, source) = setup();
    mill(
        &mut game,
        PlayerId::One,
        &[
            cards::FOREST,
            cards::GRIZZLY_BEARS,
            cards::SOL_RING,
            cards::LIGHTNING_BOLT,
            cards::PONDER,
        ],
    );
    assert_eq!(triggers(&game, source), 1);
    assert_eq!(counters(&game), 6, "removal waits for the stack");
    mill(&mut game, PlayerId::One, &[cards::MOUNTAIN]);
    assert_eq!(
        triggers(&game, source),
        2,
        "separate instructions remain separate events"
    );
    settle(&mut game);
    assert_eq!(counters(&game), 4);
}

#[test]
fn moonshadow_ignores_nonpermanents_and_opponents_graveyard() {
    let (mut game, source) = setup();
    mill(
        &mut game,
        PlayerId::One,
        &[cards::LIGHTNING_BOLT, cards::PONDER],
    );
    mill(
        &mut game,
        PlayerId::Two,
        &[cards::FOREST, cards::GRIZZLY_BEARS],
    );
    assert_eq!(triggers(&game, source), 0);
    assert_eq!(counters(&game), 6);
}

#[test]
fn moonshadow_discard_is_one_group_and_uses_card_owner() {
    let (mut game, source) = setup();
    game.set_hand(
        PlayerId::One,
        &[cards::FOREST, cards::GRIZZLY_BEARS, cards::PONDER],
    )
    .unwrap();
    let hand = game.players[0]
        .hand
        .iter()
        .map(|card| card.id)
        .collect::<Vec<_>>();
    game.discard_cards_with_cause(
        PlayerId::One,
        &hand,
        ZoneMoveCause::Effect {
            controller: PlayerId::Two,
        },
    );
    assert_eq!(triggers(&game, source), 1);
    settle(&mut game);
    assert_eq!(counters(&game), 5);
}

#[test]
fn moonshadow_simultaneous_deaths_are_one_group_and_own_death_does_not_trigger() {
    let (mut game, source) = setup();
    let bear = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .unwrap();
    let elf = game
        .put_onto_battlefield(PlayerId::One, cards::LLANOWAR_ELVES)
        .unwrap();
    game.destroy_permanents(&[bear, elf], false);
    assert_eq!(triggers(&game, source), 1);
    settle(&mut game);
    assert_eq!(counters(&game), 5);
    let bear = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .unwrap();
    game.destroy_permanents(&[source, bear], false);
    assert_eq!(
        triggers(&game, source),
        0,
        "from-anywhere triggers observe after the move"
    );
}

#[test]
fn moonshadow_from_anywhere_includes_exile_command_and_countered_permanent_spells() {
    for from in [ZoneKind::Exile, ZoneKind::Command] {
        let (mut game, source) = setup();
        let card = game
            .build_zone(PlayerId::One, &[cards::FOREST])
            .unwrap()
            .remove(0);
        let id = card.id;
        match from {
            ZoneKind::Exile => game.players[0].exile.push(card),
            ZoneKind::Command => game.players[0].command.push(card),
            _ => unreachable!(),
        }
        game.move_card_from_nonbattlefield_zone(
            id,
            from,
            ZoneKind::Graveyard,
            ZoneMoveCause::Rules,
            None,
        )
        .unwrap();
        assert_eq!(triggers(&game, source), 1, "{from:?}");
    }
    let (mut game, source) = setup();
    let held = held(&mut game, cards::GRIZZLY_BEARS);
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == held))
        .unwrap();
    game.apply(PlayerId::One, action).unwrap();
    let spell = game.stack.last().unwrap().id;
    game.counter_spell(spell);
    assert_eq!(triggers(&game, source), 1);
    settle(&mut game);
    assert_eq!(counters(&game), 5);
}

#[test]
fn moonshadow_while_is_checked_at_the_event_and_queued_triggers_survive_zero_counters() {
    let (mut game, source) = setup();
    game.battlefield
        .iter_mut()
        .find(|p| p.card.id == source)
        .unwrap()
        .counters
        .set(CounterKind::MinusOneMinusOne, 1);
    mill(&mut game, PlayerId::One, &[cards::FOREST]);
    mill(&mut game, PlayerId::One, &[cards::FOREST]);
    assert_eq!(triggers(&game, source), 2);
    assert!(
        game.pending_triggers
            .iter()
            .all(|trigger| trigger.condition.is_none())
    );
    settle(&mut game);
    assert_eq!(counters(&game), 0);
    mill(&mut game, PlayerId::One, &[cards::FOREST]);
    assert_eq!(triggers(&game, source), 0);
    game.battlefield
        .iter_mut()
        .find(|p| p.card.id == source)
        .unwrap()
        .counters
        .add(CounterKind::MinusOneMinusOne, 1);
    assert_eq!(
        triggers(&game, source),
        0,
        "adding a counter cannot retroactively trigger"
    );
}

#[test]
fn moonshadow_graveyard_replacement_prevents_the_trigger() {
    let (mut game, source) = setup();
    game.put_onto_battlefield(PlayerId::One, cards::REST_IN_PEACE)
        .unwrap();
    settle(&mut game);
    mill(
        &mut game,
        PlayerId::One,
        &[cards::FOREST, cards::GRIZZLY_BEARS],
    );
    assert_eq!(triggers(&game, source), 0);
    assert_eq!(game.players[0].exile.len(), 2);
    assert_eq!(counters(&game), 6);
}

#[test]
fn moonshadow_tokens_do_not_count_but_stolen_owned_cards_do() {
    let (mut game, source) = setup();
    cast(&mut game, cards::RAISE_THE_ALARM);
    let tokens = game
        .battlefield
        .iter()
        .filter(|p| p.card.definition.is_token())
        .map(|p| p.card.id)
        .collect::<Vec<_>>();
    assert_eq!(tokens.len(), 2);
    game.destroy_permanents(&tokens, false);
    assert_eq!(triggers(&game, source), 0);
    let ours = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .unwrap();
    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::LLANOWAR_ELVES)
        .unwrap();
    game.battlefield
        .iter_mut()
        .find(|p| p.card.id == ours)
        .unwrap()
        .controller = PlayerId::Two;
    game.battlefield
        .iter_mut()
        .find(|p| p.card.id == theirs)
        .unwrap()
        .controller = PlayerId::One;
    game.destroy_permanents(&[theirs], false);
    assert_eq!(triggers(&game, source), 0);
    game.destroy_permanents(&[ours], false);
    assert_eq!(triggers(&game, source), 1);
}

#[test]
fn moonshadow_cycling_a_permanent_card_triggers_before_the_draw_resolves() {
    let (mut game, source) = setup();
    game.set_library(PlayerId::One, &[cards::FOREST]).unwrap();
    let wraith = held(&mut game, cards::STREET_WRAITH);
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == wraith),
        )
        .unwrap();
    game.apply(PlayerId::One, action).unwrap();
    assert!(
        game.stack
            .iter()
            .any(|object| object.source == Some(source))
    );
    assert_eq!(counters(&game), 6);
    settle(&mut game);
    assert_eq!(counters(&game), 5);
    assert_eq!(game.players[0].hand.len(), 1);
}

#[test]
fn moonshadow_trigger_does_not_follow_a_new_incarnation() {
    let (mut game, source) = setup();
    mill(&mut game, PlayerId::One, &[cards::FOREST]);
    game.return_permanent_to_hand(source);
    let returned = game.players[0]
        .hand
        .iter()
        .find(|card| card.definition == cards::MOONSHADOW)
        .unwrap()
        .id;
    game.move_card_target_to_zone(
        returned,
        ZoneKind::Battlefield,
        ZoneMoveCause::Rules,
        None,
        ZonePlacement::Top,
    )
    .unwrap();
    settle(&mut game);
    assert_eq!(counters(&game), 6);
    assert_ne!(permanent(&game, cards::MOONSHADOW).card.id, source);
}

#[test]
fn moonshadow_grouped_trigger_round_trips_through_checkpoint() {
    let (mut game, _) = setup();
    mill(
        &mut game,
        PlayerId::One,
        &[cards::FOREST, cards::GRIZZLY_BEARS],
    );
    game.finish_rules_procedure();
    assert_eq!(game.stack.len(), 1);
    for viewer in [PlayerId::One, PlayerId::Two] {
        let (wire, hidden) = checkpoint_fixture(&game, viewer);
        let mut restored = Game::from_observation_checkpoint(
            game.catalog.clone(),
            game.format,
            &wire,
            &hidden,
            42,
        )
        .unwrap();
        settle(&mut restored);
        assert_eq!(counters(&restored), 5);
    }
}

#[test]
fn moonshadow_group_respects_death_trigger_modifiers_for_every_matching_member() {
    let (mut game, source) = setup();
    game.put_onto_battlefield(PlayerId::One, cards::GANDALF_THE_WHITE)
        .unwrap();
    let bear = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .unwrap();
    let artifact = game
        .put_onto_battlefield(PlayerId::One, cards::SOL_RING)
        .unwrap();
    game.destroy_permanents(&[bear, artifact], false);
    assert_eq!(
        triggers(&game, source),
        2,
        "the later artifact causes the group to trigger an additional time"
    );
    settle(&mut game);
    assert_eq!(counters(&game), 4);
}

#[test]
fn moonshadow_explore_publishes_arrival_and_honors_replacement() {
    for replaced in [false, true] {
        let (mut game, source) = setup();
        if replaced {
            game.put_onto_battlefield(PlayerId::One, cards::REST_IN_PEACE)
                .unwrap();
            settle(&mut game);
        }
        game.set_library(PlayerId::One, &[cards::GRIZZLY_BEARS])
            .unwrap();
        let revealed = game.players[0].library.last().unwrap().id;
        game.place_explored_card(PlayerId::One, revealed, true);
        assert_eq!(triggers(&game, source), usize::from(!replaced));
        assert_eq!(game.players[0].exile.len(), usize::from(replaced));
    }
}

#[test]
fn moonshadow_failed_aura_resolves_into_graveyard_and_triggers() {
    let (mut game, source) = setup();
    let target = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .unwrap();
    let aura = held(&mut game, cards::CONTROL_MAGIC);
    let action = game.legal_actions(PlayerId::One).into_iter().find(|action| matches!(action, Action::CastSpell { card, choices, .. } if *card == aura && choices.iter_targets().any(|choice| *choice == Target::Permanent(target)))).unwrap();
    game.apply(PlayerId::One, action).unwrap();
    game.destroy_permanents(&[target], false);
    assert_eq!(triggers(&game, source), 0);
    settle(&mut game);
    assert_eq!(counters(&game), 5);
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::CONTROL_MAGIC)
    );
}

#[test]
fn moonshadow_collection_move_publishes_one_group_and_keeps_its_continuation() {
    let (mut game, source) = setup();
    game.set_hand(PlayerId::One, &[cards::FOREST, cards::GRIZZLY_BEARS])
        .unwrap();
    let spell = spell(100_000, cards::TOME_SCOUR, PlayerId::One, 0);
    game.resolve_effect_def(
        ScopedEffect::primary(EffectDef::MoveObjects(crate::card::MoveObjectsDef {
            input: ObjectSetDef::Query(crate::card::ObjectQueryDef::matching(
                ObjectPredicateDef::Any,
                &[ZoneKind::Hand],
                PlayerRelation::You,
            )),
            from: Some(ZoneKind::Hand),
            zone: ZoneKind::Graveyard,
            placement: ZonePlacement::Top,
            moved: None,
            then: &EffectDef::GainLife {
                recipient: EffectRecipientDef::Controller,
                amount: ValueDef::Constant(1),
            },
        })),
        &spell,
        EffectResolutionContext::empty(),
    );
    assert_eq!(triggers(&game, source), 1);
    assert_eq!(game.players[0].graveyard.len(), 2);
    assert_eq!(game.players[0].life, 21);
    settle(&mut game);
    assert_eq!(counters(&game), 5);
}
