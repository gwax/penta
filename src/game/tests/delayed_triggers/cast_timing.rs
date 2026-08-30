use crate::CastTimingPermissionDef;

fn grant_next_sorcery_flash(game: &mut Game) {
    const PERMISSION: AppliedEffectDef =
        AppliedEffectDef::Rule(AppliedRuleDef::MayCastAsThoughItHadFlash(
            CastTimingPermissionDef::new(ObjectPredicateDef::HasType(CardType::Sorcery)),
        ));
    let source = spell(99_001, cards::QUICKEN, PlayerId::One, 0);
    game.resolve_effect_def(
        ScopedEffect::primary(EffectDef::Apply {
            recipient: EffectRecipientDef::Controller,
            effect: PERMISSION,
            duration: ResolvedEffectDurationDef::UntilEndOfTurn
                .or(ResolvedEffectDurationDef::UntilNextMatchingCast),
        }),
        &source,
        TriggerContext::empty(),
    );
}

#[test]
fn quicken_allows_endless_suspend_actions_without_expiring_its_timing_permission() {
    let mut game = ready_game();
    let quicken = card(10_020, cards::QUICKEN, PlayerId::One);
    let strobes = [
        card(10_021, cards::REALITY_STROBE, PlayerId::One),
        card(10_022, cards::REALITY_STROBE, PlayerId::One),
    ];
    game.players[0].hand.push(quicken.clone());
    game.players[0].hand.extend(strobes.iter().cloned());
    game.players[0].mana_pool.blue = 7;
    game.active_player = PlayerId::Two;
    game.priority = PlayerId::One;
    game.step = Step::PrecombatMain;

    game.apply(
        PlayerId::One,
        cast_action(quicken.id, Vec::new(), Vec::new(), 0),
    )
    .unwrap();
    pass_priority_pair(&mut game);
    game.priority = PlayerId::One;
    assert_eq!(game.resolved_play_permissions.len(), 1);

    for strobe in &strobes {
        let suspend = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| matches!(action, Action::Suspend { card, .. } if *card == strobe.id))
            .expect("Quicken opens the suspend action for every sorcery card");
        game.apply(PlayerId::One, suspend).unwrap();
        assert_eq!(
            game.resolved_play_permissions.len(),
            1,
            "suspend is not a cast and does not expire Quicken"
        );
    }

    let suspended = game.players[0]
        .exile
        .iter()
        .filter(|card| card.definition == cards::REALITY_STROBE)
        .collect::<Vec<_>>();
    assert_eq!(suspended.len(), 2);
    assert!(
        suspended
            .iter()
            .all(|card| card.counters(CounterKind::named("time")) == 3)
    );

    let actual_sorcery = card(10_023, cards::MIND_TWIST, PlayerId::One);
    game.players[0].hand.push(actual_sorcery.clone());
    game.players[0].mana_pool.black = 1;
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(
            |action| matches!(action, Action::CastSpell { card, .. } if *card == actual_sorcery.id),
        )
        .expect("Quicken remains for the next sorcery spell actually cast");
    game.apply(PlayerId::One, cast).unwrap();
    assert!(game.resolved_play_permissions.is_empty());
}
