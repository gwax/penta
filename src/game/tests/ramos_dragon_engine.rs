use super::*;

fn board(prepared: bool) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.set_prepared_engine_enabled(prepared);
    let ramos = game
        .put_onto_battlefield(PlayerId::One, cards::RAMOS_DRAGON_ENGINE)
        .unwrap();
    (game, ramos)
}

fn counters(game: &Game, ramos: GameObjectId) -> u16 {
    game.battlefield
        .iter()
        .find(|p| p.card.id == ramos)
        .unwrap()
        .counters(CounterKind::PlusOnePlusOne)
}

fn cast(game: &mut Game, player: PlayerId, definition: CardDefinitionId) -> GameObjectId {
    let held = game.build_zone(player, &[definition]).unwrap().remove(0);
    let id = held.id;
    game.players[player.index()].hand.push(held);
    for color in ManaColor::COLORS {
        game.add_unrestricted_mana(player, color, 4);
    }
    game.active_player = player;
    game.priority = player;
    let action = game
        .legal_actions(player)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == id))
        .unwrap();
    game.apply(player, action).unwrap();
    game.stack
        .iter()
        .find(|object| object.kind == StackObjectKind::Spell)
        .unwrap()
        .id
}

fn mana_action(game: &Game, ramos: GameObjectId) -> Option<Action> {
    game.legal_actions(PlayerId::One).into_iter().find(
        |action| matches!(action, Action::ActivateManaAbility { source, .. } if *source == ramos),
    )
}

#[test]
fn ramos_counts_spell_colors_only_for_its_controllers_casts() {
    for prepared in [false, true] {
        for player in [PlayerId::One, PlayerId::Two] {
            for (definition, colors) in [
                (cards::ORNITHOPTER, 0),
                (cards::GRIZZLY_BEARS, 1),
                (cards::LOXODON_SMITER, 2),
            ] {
                let (mut game, ramos) = board(prepared);
                cast(&mut game, player, definition);
                let mine = player == PlayerId::One;
                assert_eq!(game.stack.len(), if mine { 2 } else { 1 });
                assert_eq!(counters(&game, ramos), 0, "counters wait for resolution");
                if mine {
                    game.resolve_stack_top();
                    assert_eq!(counters(&game, ramos), colors);
                    assert_eq!(game.stack.len(), 1, "Ramos resolves before the spell");
                }
            }
        }
    }
}

#[test]
fn ramos_reads_live_spell_colors_instead_of_freezing_the_cast_event() {
    for prepared in [false, true] {
        for count in 0..=5 {
            let (mut game, ramos) = board(prepared);
            let spell = cast(&mut game, PlayerId::One, cards::ORNITHOPTER);
            // The state produced by a color-changing effect on the stack.
            game.stack
                .iter_mut()
                .find(|object| object.id == spell)
                .unwrap()
                .colors = Some(ColorSet::from_colors(&ManaColor::COLORS[..count]));
            game.resolve_stack_top();
            assert_eq!(counters(&game, ramos), u16::try_from(count).unwrap());
        }
    }
}

#[test]
fn ramos_keeps_a_countered_spells_last_known_colors_through_reconstruction() {
    for prepared in [false, true] {
        let (mut game, ramos) = board(prepared);
        let spell = cast(&mut game, PlayerId::One, cards::ORNITHOPTER);
        game.stack
            .iter_mut()
            .find(|object| object.id == spell)
            .unwrap()
            .colors = Some(ColorSet::from_colors(&[
            ManaColor::Blue,
            ManaColor::Red,
            ManaColor::Green,
        ]));
        game.counter_spell(spell);
        assert_eq!(
            game.stack.len(),
            1,
            "countering the spell leaves Ramos's trigger"
        );
        assert_eq!(game.object_color_count(spell), 3);
        let (wire, hidden) = checkpoint_fixture(&game, PlayerId::One);
        let mut rebuilt =
            Game::from_observation_checkpoint(game.catalog.clone(), game.format, &wire, &hidden, 0)
                .unwrap();
        rebuilt.set_prepared_engine_enabled(prepared);
        game.resolve_stack_top();
        rebuilt.resolve_stack_top();
        assert_eq!(counters(&game, ramos), 3);
        assert_eq!(counters(&rebuilt, ramos), 3);
    }
}

#[test]
fn ramos_mana_pays_five_counters_and_remembers_its_once_each_turn_limit() {
    for prepared in [false, true] {
        let (mut game, ramos) = board(prepared);
        game.battlefield
            .iter_mut()
            .find(|p| p.card.id == ramos)
            .unwrap()
            .set_counters(CounterKind::PlusOnePlusOne, 4);
        assert!(mana_action(&game, ramos).is_none());
        game.battlefield
            .iter_mut()
            .find(|p| p.card.id == ramos)
            .unwrap()
            .set_counters(CounterKind::PlusOnePlusOne, 10);
        let action = mana_action(&game, ramos).unwrap();
        game.apply(PlayerId::One, action).unwrap();
        assert!(
            game.stack.is_empty(),
            "the mana ability resolves immediately"
        );
        assert_eq!(counters(&game, ramos), 5);
        assert_eq!(
            game.players[0].mana_pool,
            ManaPool {
                white: 2,
                blue: 2,
                black: 2,
                red: 2,
                green: 2,
                ..ManaPool::default()
            }
        );
        assert!(mana_action(&game, ramos).is_none());
        let (wire, hidden) = checkpoint_fixture(&game, PlayerId::One);
        let mut rebuilt =
            Game::from_observation_checkpoint(game.catalog.clone(), game.format, &wire, &hidden, 0)
                .unwrap();
        rebuilt.set_prepared_engine_enabled(prepared);
        assert!(mana_action(&rebuilt, ramos).is_none());
        rebuilt.start_next_turn();
        rebuilt.priority = PlayerId::One;
        assert_eq!(rebuilt.active_player, PlayerId::Two);
        let action = mana_action(&rebuilt, ramos).expect("available again on the opponent's turn");
        rebuilt.apply(PlayerId::One, action).unwrap();
        assert_eq!(counters(&rebuilt, ramos), 0);
    }
}

#[test]
fn ramos_cannot_fund_a_spell_with_counters_from_its_pending_cast_trigger() {
    let (mut game, ramos) = board(true);
    game.battlefield
        .iter_mut()
        .find(|p| p.card.id == ramos)
        .unwrap()
        .set_counters(CounterKind::PlusOnePlusOne, 4);
    let held = game
        .build_zone(PlayerId::One, &[cards::GRIZZLY_BEARS])
        .unwrap()
        .remove(0);
    let id = held.id;
    game.players[0].hand.push(held);
    let cast_action = |game: &Game| {
        game.legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == id))
    };
    assert!(
        cast_action(&game).is_none(),
        "a future fifth counter cannot pay for this cast"
    );
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 2);
    game.apply(PlayerId::One, cast_action(&game).unwrap())
        .unwrap();
    assert_eq!(counters(&game, ramos), 4);
    assert!(mana_action(&game, ramos).is_none());
    game.resolve_stack_top();
    assert_eq!(counters(&game, ramos), 5);
    assert!(mana_action(&game, ramos).is_some());
}
