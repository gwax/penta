//! Text-sensitive compositions promoted by the Nemesis audit.
use super::*;

fn staged() -> Game {
    let mut game = ready_game();
    game.turns_started = [5, 5];
    for player in [PlayerId::One, PlayerId::Two] {
        for color in [
            ManaColor::White,
            ManaColor::Blue,
            ManaColor::Black,
            ManaColor::Red,
            ManaColor::Green,
        ] {
            game.add_unrestricted_mana(player, color, 3);
        }
    }
    game
}

fn permanent(game: &Game, id: GameObjectId) -> &Permanent {
    game.battlefield
        .iter()
        .find(|p| p.card.id == id)
        .expect("permanent remains")
}

fn cast(game: &mut Game, definition: CardDefinitionId, target: Option<GameObjectId>) {
    let instance = card(
        90_000 + u32::try_from(game.players[0].graveyard.len()).expect("small fixture"),
        definition,
        PlayerId::One,
    );
    let id = instance.id;
    game.players[0].hand.push(instance);
    game.priority = PlayerId::One;
    let action = game.legal_actions(PlayerId::One).into_iter().find(|action| matches!(action,
        Action::CastSpell {card, choices, ..} if *card == id && target.is_none_or(|id|
            choices.targets().iter().flat_map(TargetSelection::targets).any(|t| *t == Target::Permanent(id)))))
        .expect("spell is legal");
    game.apply(PlayerId::One, action).unwrap();
}

fn activate(game: &mut Game, source: GameObjectId, target: Option<GameObjectId>, chosen_x: u16) {
    game.priority = PlayerId::One;
    let action = game.legal_actions(PlayerId::One).into_iter().find(|action| matches!(action,
        Action::ActivateAbility {source: id, targets, x, ..} if *id == source && *x == chosen_x && target.is_none_or(|id|
            targets.iter().flat_map(TargetSelection::targets).any(|t| *t == Target::Permanent(id)))))
        .expect("activation is legal");
    game.apply(PlayerId::One, action).unwrap();
}

#[test]
fn avenger_uses_the_exiled_attackers_last_controller_and_toughness() {
    let mut game = staged();
    let source = game
        .put_onto_battlefield(PlayerId::One, cards::AVENGER_EN_DAL)
        .unwrap();
    let victim = game
        .put_onto_battlefield(PlayerId::One, cards::SERRA_ANGEL)
        .unwrap();
    for p in &mut game.battlefield {
        p.entered_controller_turn = 0;
        if p.card.id == victim {
            p.controller = PlayerId::Two;
            p.attacking = true;
            p.attack_defender = Some(AttackDefender::Player(PlayerId::One));
        }
    }
    game.players[0]
        .hand
        .push(card(91_000, cards::MOUNTAIN, PlayerId::One));
    activate(&mut game, source, Some(victim), 0);
    drain_pending(&mut game);
    assert!(
        game.players[0]
            .exile
            .iter()
            .any(|c| c.definition == cards::SERRA_ANGEL)
    );
    assert_eq!(game.players[0].life, 20);
    assert_eq!(game.players[1].life, 24);
}

#[test]
fn lin_sivvi_search_uses_announced_x_and_filters_larger_rebels() {
    let mut game = staged();
    let sivvi = game
        .put_onto_battlefield(PlayerId::One, cards::LIN_SIVVI_DEFIANT_HERO)
        .unwrap();
    game.battlefield
        .iter_mut()
        .for_each(|p| p.entered_controller_turn = 0);
    game.players[0].library = vec![
        card(91_001, cards::DEFIANT_FALCON, PlayerId::One),
        card(91_002, cards::LIGHTBRINGER, PlayerId::One),
    ];
    activate(&mut game, sivvi, None, 2);
    drain_pending(&mut game);
    assert!(
        game.battlefield
            .iter()
            .any(|p| p.card.definition == cards::DEFIANT_FALCON)
    );
    assert_eq!(game.players[0].library.len(), 1);
    assert_eq!(game.players[0].library[0].definition, cards::LIGHTBRINGER);
}

#[test]
fn stronghold_gambit_ignores_noncreatures_and_returns_tied_creatures_to_their_owners() {
    for own in [cards::LIGHTNING_BOLT, cards::GRIZZLY_BEARS] {
        let mut game = staged();
        game.players[0].hand.push(card(91_010, own, PlayerId::One));
        game.players[1]
            .hand
            .push(card(91_011, cards::GRIZZLY_BEARS, PlayerId::Two));
        cast(&mut game, cards::STRONGHOLD_GAMBIT, None);
        drain_pending(&mut game);
        assert!(game.battlefield.iter().any(|p| p.card.definition == cards::GRIZZLY_BEARS && p.controller == PlayerId::Two));
        if own == cards::LIGHTNING_BOLT {
            assert_eq!(game.battlefield.len(), 1);
            assert_eq!(game.players[0].hand[0].definition, cards::LIGHTNING_BOLT);
        } else {
            assert_eq!(game.battlefield.len(), 2);
            assert!(
                game.battlefield
                    .iter()
                    .any(|p| p.controller == PlayerId::One)
            );
        }
    }
}

#[test]
fn burst_tracks_live_counters_and_destroys_only_its_tokens_even_after_they_change_control() {
    let mut game = staged();
    let first = game
        .put_onto_battlefield(PlayerId::One, cards::SAPROLING_BURST)
        .unwrap();
    let second = game
        .put_onto_battlefield(PlayerId::One, cards::SAPROLING_BURST)
        .unwrap();
    activate(&mut game, first, None, 0);
    drain_pending(&mut game);
    activate(&mut game, first, None, 0);
    drain_pending(&mut game);
    activate(&mut game, second, None, 0);
    drain_pending(&mut game);
    let tokens: Vec<_> = game
        .battlefield
        .iter()
        .filter(|p| p.created_by == Some(first))
        .map(|p| p.card.id)
        .collect();
    assert_eq!(tokens.len(), 2);
    for id in &tokens {
        assert_eq!(game.power(permanent(&game, *id)), Some(5));
        game.battlefield
            .iter_mut()
            .find(|p| p.card.id == *id)
            .unwrap()
            .controller = PlayerId::Two;
    }
    game.move_permanents_to_graveyard(&[first]);
    drain_pending(&mut game);
    assert!(
        game.battlefield
            .iter()
            .all(|p| !tokens.contains(&p.card.id))
    );
    let token = game
        .battlefield
        .iter()
        .find(|p| p.created_by == Some(second))
        .unwrap();
    assert_eq!(game.power(token), Some(6));
}

#[test]
fn inhibitor_checks_fading_abilities_instead_of_existing_fade_counters() {
    let mut game = staged();
    let inhibitor = game
        .put_onto_battlefield(PlayerId::One, cards::PARALLAX_INHIBITOR)
        .unwrap();
    let fading = game
        .put_onto_battlefield(PlayerId::One, cards::BLASTODERM)
        .unwrap();
    let ordinary = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .unwrap();
    let counter = CounterKind::named("fade");
    game.battlefield
        .iter_mut()
        .find(|p| p.card.id == ordinary)
        .unwrap()
        .set_counters(counter, 2);
    game.battlefield
        .iter_mut()
        .find(|p| p.card.id == fading)
        .unwrap()
        .set_counters(counter, 0);
    activate(&mut game, inhibitor, None, 0);
    drain_pending(&mut game);
    assert_eq!(permanent(&game, fading).counters(counter), 1);
    assert_eq!(permanent(&game, ordinary).counters(counter), 2);
}

#[test]
fn volrath_uses_the_discarded_cards_mana_value_after_it_leaves_hand() {
    let mut game = staged();
    let volrath = game
        .put_onto_battlefield(PlayerId::One, cards::VOLRATH_THE_FALLEN)
        .unwrap();
    game.players[0]
        .hand
        .push(card(91_020, cards::SERRA_ANGEL, PlayerId::One));
    activate(&mut game, volrath, None, 0);
    drain_pending(&mut game);
    assert_eq!(game.power(permanent(&game, volrath)), Some(11));
    assert_eq!(game.toughness(permanent(&game, volrath)), Some(9));
    assert_eq!(game.players[0].graveyard[0].definition, cards::SERRA_ANGEL);
}

#[test]
fn rootwater_thiefs_controller_searches_the_damaged_players_library() {
    let mut game = staged();
    let thief = game
        .put_onto_battlefield(PlayerId::One, cards::ROOTWATER_THIEF)
        .unwrap();
    game.players[1].library = vec![card(92_100, cards::LIGHTNING_BOLT, PlayerId::Two)];
    let p = game
        .battlefield
        .iter_mut()
        .find(|p| p.card.id == thief)
        .unwrap();
    p.attacking = true;
    p.attack_defender = Some(AttackDefender::Player(PlayerId::Two));
    game.step = Step::DeclareBlockers;
    game.attackers_declared = true;
    game.finish_declaring_blockers();
    game.deal_combat_damage();
    game.finish_rules_procedure();
    pass_until_decision(&mut game);
    let payment = game
        .pending_decisions
        .first()
        .expect("optional payment")
        .observation
        .clone();
    assert_eq!(payment.player, PlayerId::One);
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: payment.id,
            options: vec![
                payment
                    .options
                    .iter()
                    .find(|o| o.label.starts_with("Pay"))
                    .expect("can pay")
                    .id,
            ],
        },
    )
    .unwrap();
    let search = game
        .pending_decisions
        .first()
        .expect("library search")
        .observation
        .clone();
    assert_eq!(search.player, PlayerId::One);
    assert!(search.options.iter().any(|o| o.label == "Lightning Bolt"));
    drain_pending(&mut game);
    assert!(game.players[1].library.is_empty());
    assert_eq!(game.players[1].exile[0].definition, cards::LIGHTNING_BOLT);
}

#[test]
fn laccolith_rig_retains_the_blocked_creature_after_the_aura_leaves() {
    let mut game = staged();
    let attacker = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .unwrap();
    let wall = game
        .put_onto_battlefield(PlayerId::Two, cards::WALL_OF_WOOD)
        .unwrap();
    let victim = game
        .put_onto_battlefield(PlayerId::Two, cards::SERRA_ANGEL)
        .unwrap();
    cast(&mut game, cards::LACCOLITH_RIG, Some(attacker));
    drain_pending(&mut game);
    let aura = game
        .battlefield
        .iter()
        .find(|p| p.card.definition == cards::LACCOLITH_RIG)
        .unwrap()
        .card
        .id;
    for p in &mut game.battlefield {
        if p.card.id == attacker {
            p.attacking = true;
            p.attack_defender = Some(AttackDefender::Player(PlayerId::Two));
        }
        if p.card.id == wall {
            p.blocking = vec![attacker];
        }
    }
    game.step = Step::DeclareBlockers;
    game.attackers_declared = true;
    game.finish_declaring_blockers();
    game.move_permanents_to_graveyard(&[aura]);
    for _ in 0..16 {
        if let Some(pending) = game.pending_decisions.first() {
            let d = pending.observation.clone();
            let choice = d
                .options
                .iter()
                .find(|o| o.label == "Serra Angel" || o.label == "Do it")
                .expect("target or optional damage")
                .id;
            game.apply(
                d.player,
                Action::ChooseDecision {
                    decision: d.id,
                    options: vec![choice],
                },
            )
            .unwrap();
        } else if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        } else {
            let priority = game.priority;
            game.apply(priority, Action::PassPriority).unwrap();
        }
    }
    assert_eq!(permanent(&game, victim).damage, 2);
    game.deal_combat_damage();
    assert_eq!(permanent(&game, wall).damage, 0);
}

#[test]
fn wild_mammoth_rechecks_a_changed_majority_before_changing_control() {
    for tie_on_resolution in [false, true] {
        let mut game = staged();
        let mammoth = game
            .put_onto_battlefield(PlayerId::One, cards::WILD_MAMMOTH)
            .unwrap();
        game.put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
            .unwrap();
        game.put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
            .unwrap();
        game.step = Step::Upkeep;
        game.handle_upkeep_triggers();
        game.finish_rules_procedure();
        if tie_on_resolution {
            game.put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
                .unwrap();
        }
        drain_pending(&mut game);
        assert_eq!(
            permanent(&game, mammoth).controller,
            if tie_on_resolution {
                PlayerId::One
            } else {
                PlayerId::Two
            }
        );
    }
}

#[test]
fn parallax_dementia_destroys_its_last_attached_creature_when_it_leaves() {
    let mut game = staged();
    let bear = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .unwrap();
    cast(&mut game, cards::PARALLAX_DEMENTIA, Some(bear));
    drain_pending(&mut game);
    assert_eq!(game.power(permanent(&game, bear)), Some(5));
    let aura = game
        .battlefield
        .iter()
        .find(|p| p.card.definition == cards::PARALLAX_DEMENTIA)
        .unwrap()
        .card
        .id;
    game.move_permanents_to_graveyard(&[aura]);
    drain_pending(&mut game);
    assert!(
        game.players[1]
            .graveyard
            .iter()
            .any(|c| c.definition == cards::GRIZZLY_BEARS)
    );
}

#[test]
fn parallax_nexus_keeps_unchosen_hand_cards_private_and_returns_its_exile() {
    let mut game = staged();
    let nexus = game
        .put_onto_battlefield(PlayerId::One, cards::PARALLAX_NEXUS)
        .unwrap();
    game.players[1].hand = vec![
        card(93_000, cards::GRIZZLY_BEARS, PlayerId::Two),
        card(93_001, cards::LIGHTNING_BOLT, PlayerId::Two),
    ];
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|a| matches!(a, Action::ActivateAbility {source, ..} if *source == nexus))
        .unwrap();
    game.apply(PlayerId::One, action).unwrap();
    pass_priority_pair(&mut game);
    let decision = game
        .pending_decisions
        .first()
        .expect("opponent chooses")
        .observation
        .clone();
    assert_eq!(decision.player, PlayerId::Two);
    assert!(
        game.observe(PlayerId::One)
            .decision
            .as_ref()
            .is_none_or(|d| d.options.is_empty())
    );
    game.apply(
        PlayerId::Two,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![decision.options[0].id],
        },
    )
    .unwrap();
    drain_pending(&mut game);
    assert_eq!(game.players[1].hand.len(), 1);
    assert_eq!(game.players[1].exile.len(), 1);
    game.move_permanents_to_graveyard(&[nexus]);
    drain_pending(&mut game);
    assert_eq!(game.players[1].hand.len(), 2);
    assert!(game.players[1].exile.is_empty());
}
