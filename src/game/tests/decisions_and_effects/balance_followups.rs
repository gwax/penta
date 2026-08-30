#[test]
fn balance_recounts_creatures_after_loxodon_smiter_replaces_its_discard() {
    let mut game = ready_game();
    let balance = card(10_010, cards::BALANCE, PlayerId::One);
    game.players[0].hand.push(balance.clone());
    game.players[1]
        .hand
        .push(card(10_011, cards::LOXODON_SMITER, PlayerId::Two));
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);

    game.apply(
        PlayerId::One,
        cast_action(balance.id, Vec::new(), Vec::new(), 0),
    )
    .unwrap();
    pass_priority_pair(&mut game);

    let discard = game
        .observe(PlayerId::Two)
        .decision
        .expect("Balance makes player two discard down to zero");
    assert_eq!(discard.visibility, DecisionVisibility::Private);
    let smiter = discard
        .options
        .iter()
        .find(|option| {
            option
                .card
                .is_some_and(|(_, characteristics)| characteristics.card_definition() == Some(cards::LOXODON_SMITER))
        })
        .expect("Loxodon Smiter is the discard choice")
        .id;
    game.apply(
        PlayerId::Two,
        Action::ChooseDecision {
            decision: discard.id,
            options: vec![smiter],
        },
    )
    .unwrap();

    let sacrifice = game
        .observe(PlayerId::Two)
        .decision
        .expect("the creature step is counted after the discard step");
    assert_eq!(sacrifice.visibility, DecisionVisibility::Public);
    assert!(sacrifice.prompt.contains("creature"));
    assert_eq!(sacrifice.options.len(), 1);
    assert!(
        sacrifice.options[0]
            .card
            .is_some_and(|(_, characteristics)| characteristics.card_definition() == Some(cards::LOXODON_SMITER))
    );
    game.apply(
        PlayerId::Two,
        Action::ChooseDecision {
            decision: sacrifice.id,
            options: vec![sacrifice.options[0].id],
        },
    )
    .unwrap();

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.definition != cards::LOXODON_SMITER)
    );
    assert_eq!(
        game.players[1]
            .graveyard
            .iter()
            .filter(|card| card.definition == cards::LOXODON_SMITER)
            .count(),
        1,
    );
    assert!(game.events.iter().any(|event| matches!(
        event,
        GameEvent::CardsDiscarded {
            player: PlayerId::Two,
            cards,
        } if cards.iter().any(|(_, definition)| *definition == cards::LOXODON_SMITER)
    )));
}

#[test]
fn balance_defers_one_apnap_trigger_batch_until_its_decisions_finish() {
    let mut game = ready_game();
    game.battlefield.extend([
        creature(10_000, cards::SU_CHI, PlayerId::One),
        creature(10_001, cards::SU_CHI, PlayerId::One),
        creature(10_002, cards::SAVANNAH_LIONS, PlayerId::One),
        creature(10_003, cards::SAVANNAH_LIONS, PlayerId::Two),
    ]);
    game.players[0].hand.extend([
        card(10_004, cards::LIGHTNING_BOLT, PlayerId::One),
        card(10_005, cards::MOUNTAIN, PlayerId::One),
    ]);

    game.resolve_balance(PlayerId::One);
    let discard = game.observe(PlayerId::One).decision.unwrap();
    assert_eq!(discard.kind, DecisionKind::Choice);
    assert!(discard.prompt.contains("discard"));
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: discard.id,
            options: discard.options.iter().map(|option| option.id).collect(),
        },
    )
    .unwrap();

    let sacrifice = game.observe(PlayerId::One).decision.unwrap();
    let su_chi = sacrifice
        .options
        .iter()
        .filter(|option| {
            option
                .card
                .is_some_and(|(_, characteristics)| characteristics.card_definition() == Some(cards::SU_CHI))
        })
        .map(|option| option.id)
        .collect::<Vec<_>>();
    assert_eq!(su_chi.len(), 2);
    assert!(sacrifice.prompt.contains("creature"));
    assert!(game.stack.is_empty());
    assert!(game.pending_triggers.is_empty());
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: sacrifice.id,
            options: su_chi,
        },
    )
    .unwrap();

    let order = game.observe(PlayerId::One).decision.unwrap();
    assert_eq!(order.kind, DecisionKind::TriggerOrder);
    assert_eq!(order.options.len(), 2);
    assert!(game.stack.is_empty());
    assert!(game.pending_triggers.is_empty());

    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: order.id,
            options: order.options.iter().map(|option| option.id).collect(),
        },
    )
    .unwrap();
    assert_eq!(game.stack.len(), 2);
    assert!(
        game.stack
            .iter()
            .all(|object| object.kind == StackObjectKind::TriggeredAbility)
    );
}

/// "Balance doesn't have targets, so permanents that can't be targeted, such
/// as a creature with shroud or protection from white, are valid choices to
/// be sacrificed." A Neurok Commando is chosen and sacrificed like anything
/// else.
#[test]
fn balance_sacrifices_a_shrouded_creature_it_could_never_have_targeted() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[1].hand.clear();
    let balance = card(10_040, cards::BALANCE, PlayerId::One);
    game.players[0].hand.push(balance.clone());
    game.battlefield
        .push(creature(10_041, cards::NEUROK_COMMANDO, PlayerId::Two));
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);

    game.apply(
        PlayerId::One,
        cast_action(balance.id, Vec::new(), Vec::new(), 0),
    )
    .unwrap();
    pass_priority_pair(&mut game);

    let sacrifice = game
        .observe(PlayerId::Two)
        .decision
        .expect("their only creature is one more than your none");
    assert!(sacrifice.prompt.contains("creature"));
    assert_eq!(
        sacrifice.options.len(),
        1,
        "shroud is no reason to leave it out of the choice",
    );
    game.apply(
        PlayerId::Two,
        Action::ChooseDecision {
            decision: sacrifice.id,
            options: vec![sacrifice.options[0].id],
        },
    )
    .unwrap();

    assert!(
        game.battlefield.is_empty(),
        "and it is sacrificed like anything else",
    );
    assert_eq!(
        game.players[1]
            .graveyard
            .iter()
            .filter(|card| card.definition == cards::NEUROK_COMMANDO)
            .count(),
        1,
    );
}
