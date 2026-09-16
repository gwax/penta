use super::*;

#[test]
fn jegantha_mana_pays_colored_symbols_but_not_generic_even_with_lattice() {
    for lattice in [false, true] {
        let mut game = ready_game();
        if lattice {
            put_ready(&mut game, cards::MYCOSYNTH_LATTICE);
        }
        let elk = put_ready(&mut game, cards::JEGANTHA_THE_WELLSPRING);
        activate_mana(&mut game, elk);
        assert_eq!(game.players[0].mana_pool.total(), 5);
        assert!(!game.can_pay_cost_for(
            PlayerId::One,
            mana_cost!("{1}"),
            0,
            &ManaPaymentPurpose::Other
        ));
        assert!(game.can_pay_cost_for(
            PlayerId::One,
            mana_cost!("{W}{U}{B}{R}{G}"),
            0,
            &ManaPaymentPurpose::Other
        ));
        assert_eq!(
            game.can_pay_cost_for(
                PlayerId::One,
                mana_cost!("{B}{B}{B}{B}{B}"),
                0,
                &ManaPaymentPurpose::Other
            ),
            lattice
        );
        game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
        let spent = game.pay_player_cost(PlayerId::One, mana_cost!("{1}{B}"), 0);
        assert_eq!(spent.len(), 2);
        assert_eq!(
            spent.iter().filter(|m| m.restrictions.is_empty()).count(),
            1
        );
    }
}

#[test]
fn jegantha_cannot_pay_a_generic_half_of_twobrid_or_restricted_x() {
    let mut game = ready_game();
    put_ready(&mut game, cards::MYCOSYNTH_LATTICE);
    let elk = put_ready(&mut game, cards::JEGANTHA_THE_WELLSPRING);
    activate_mana(&mut game, elk);
    assert!(game.can_pay_cost_for(
        PlayerId::One,
        mana_cost!("{2/W}"),
        0,
        &ManaPaymentPurpose::Other
    ));
    let (cost, x) = fold_restricted_x(mana_cost!("{X}{B}"), 1, ManaColor::Black);
    assert!(!game.can_pay_cost_for(PlayerId::One, cost, x, &ManaPaymentPurpose::Other));
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    assert!(!game.can_pay_cost_for(PlayerId::One, cost, x, &ManaPaymentPurpose::Other));
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    assert!(game.can_pay_cost_for(PlayerId::One, cost, x, &ManaPaymentPurpose::Other));
}

#[test]
fn damping_sphere_replaces_only_the_tapped_lands_output() {
    let mut game = ready_game();
    put_ready(&mut game, cards::DAMPING_SPHERE);
    let tomb = put_ready(&mut game, cards::ANCIENT_TOMB);
    activate_mana(&mut game, tomb);
    assert_eq!(game.players[0].mana_pool.colorless, 1);
    assert_eq!(game.players[0].life, 18);
    let ring = put_ready(&mut game, cards::SOL_RING);
    activate_mana(&mut game, ring);
    assert_eq!(game.players[0].mana_pool.colorless, 3);
    let forest = put_ready(&mut game, cards::FOREST);
    put_ready(&mut game, cards::BADGERMOLE_CUB);
    drain_pending(&mut game);
    // Earthbend's chosen land is the first legal land (Ancient Tomb). Its
    // replacement produces C, followed by the Cub's separate G trigger.
    game.battlefield
        .iter_mut()
        .find(|p| p.card.id == tomb)
        .unwrap()
        .tapped = false;
    activate_mana(&mut game, tomb);
    assert_eq!(game.players[0].mana_pool.colorless, 4);
    assert_eq!(game.players[0].mana_pool.green, 1);
    assert!(
        !game
            .battlefield
            .iter()
            .find(|p| p.card.id == forest)
            .unwrap()
            .tapped
    );
}

#[test]
fn damping_sphere_tax_counts_spells_cast_before_it_entered() {
    let mut game = ready_game();
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    cast(
        &mut game,
        cards::LIGHTNING_BOLT,
        Some(Target::Player(PlayerId::Two)),
    );
    drain_pending(&mut game);
    put_ready(&mut game, cards::DAMPING_SPHERE);
    game.players[0].hand = game
        .build_zone(PlayerId::One, &[cards::LIGHTNING_BOLT])
        .unwrap();
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|a| matches!(a, Action::CastSpell { .. }))
    );
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    assert!(
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|a| matches!(a, Action::CastSpell { .. }))
    );
}

#[test]
fn waterbend_taps_artifacts_and_summoning_sick_creatures_and_exhausts_once() {
    let mut game = ready_game();
    let vehicle = game
        .put_onto_battlefield(PlayerId::One, cards::INVASION_SUBMERSIBLE)
        .unwrap();
    drain_pending(&mut game);
    let bear = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .unwrap();
    let relic = put_ready(&mut game, cards::DARKSTEEL_RELIC);
    activate(&mut game, vehicle);
    drain_pending(&mut game);
    for id in [vehicle, bear, relic] {
        assert!(
            game.battlefield
                .iter()
                .find(|p| p.card.id == id)
                .unwrap()
                .tapped
        );
    }
    let permanent = game
        .battlefield
        .iter()
        .find(|p| p.card.id == vehicle)
        .unwrap();
    assert_eq!(game.power(permanent), Some(3));
    assert!(game.permanent_types(permanent).unwrap().is_creature());
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 20);
    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|a| matches!(a, Action::ActivateAbility { source, .. } if *source == vehicle))
    );
}

#[test]
fn hogaak_uses_convoke_and_delve_without_spending_mana() {
    let mut game = ready_game();
    game.players[0].hand = game
        .build_zone(PlayerId::One, &[cards::HOGAAK_ARISEN_NECROPOLIS])
        .unwrap();
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 20);
    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|a| matches!(a, Action::CastSpell { .. }))
    );
    for _ in 0..2 {
        game.put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
            .unwrap();
    }
    game.players[0].graveyard = game.build_zone(PlayerId::One, &[cards::FOREST; 5]).unwrap();
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|a| matches!(a, Action::CastSpell { .. }))
        .unwrap();
    game.apply(PlayerId::One, action).unwrap();
    drain_pending(&mut game);
    assert!(
        game.battlefield
            .iter()
            .any(|p| p.card.definition == cards::HOGAAK_ARISEN_NECROPOLIS)
    );
    assert_eq!(game.players[0].mana_pool.green, 20);
    assert_eq!(game.players[0].exile.len(), 5);
}

#[test]
fn explicit_waterbend_selects_contributions_and_restores_the_payment_draft() {
    fn answer(game: &mut Game, option: u32) {
        let observation = &game.pending_decisions[0].observation;
        game.apply(
            observation.player,
            Action::ChooseDecision {
                decision: observation.id,
                options: vec![option],
            },
        )
        .unwrap();
    }
    let mut game = ready_game();
    let vehicle = put_ready(&mut game, cards::INVASION_SUBMERSIBLE);
    drain_pending(&mut game);
    let bear = put_ready(&mut game, cards::GRIZZLY_BEARS);
    let relic = put_ready(&mut game, cards::DARKSTEEL_RELIC);
    let index = game
        .manual_payment_actions(PlayerId::One)
        .iter()
        .position(|a| matches!(a, Action::ActivateAbility {source, ..} if *source == vehicle))
        .unwrap();
    game.apply(PlayerId::One, Action::BeginPayment).unwrap();
    answer(&mut game, u32::try_from(index).unwrap());
    for source in [vehicle, bear, relic] {
        let DecisionContinuation::Payment(crate::game::payment::state::PaymentDecision::Funding(
            draft,
        )) = &game.pending_decisions[0].continuation
        else {
            panic!("funding decision")
        };
        let offset = game.funding_candidates(draft).unwrap().len();
        let contribution = game
            .contribution_candidates(draft)
            .unwrap()
            .iter()
            .position(|c| c.source == source)
            .unwrap();
        answer(&mut game, u32::try_from(offset + contribution + 1).unwrap());
    }
    let (wire, hidden) = checkpoint_fixture(&game, PlayerId::One);
    let mut restored =
        Game::from_observation_checkpoint(game.catalog.clone(), game.format, &wire, &hidden, 2)
            .unwrap();
    assert_eq!(
        game.legal_actions(PlayerId::One),
        restored.legal_actions(PlayerId::One)
    );
    answer(&mut restored, 0);
    drain_pending(&mut restored);
    let permanent = restored
        .battlefield
        .iter()
        .find(|p| p.card.id == vehicle)
        .unwrap();
    assert_eq!(restored.power(permanent), Some(3));
    assert!(
        restored
            .battlefield
            .iter()
            .filter(|p| [vehicle, bear, relic].contains(&p.card.id))
            .all(|p| p.tapped)
    );
}

#[test]
fn selected_contributors_reduce_the_bill_even_when_floating_mana_could_pay_it() {
    use crate::game::payment::allocation::{PaymentPool, contribution_remainder};
    let mut pool = PaymentPool::default();
    pool.add_color(ManaColor::Green, 3);
    pool.direct.add_color(ManaColor::Green, 1);
    let remaining = contribution_remainder(pool, mana_cost!("{2}{G}"), 0).unwrap();
    assert_eq!(remaining.mana_value(), 2);
    // The chosen green creature cannot pay a blue symbol even when the mana
    // can be spent as any color. Its contribution pays the generic symbol.
    pool.any_color = true;
    assert_eq!(
        contribution_remainder(pool, mana_cost!("{1}{U}"), 0),
        Some(mana_cost!("{U}"))
    );
    assert!(contribution_remainder(pool, mana_cost!("{U}"), 0).is_none());
}
