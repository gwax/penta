//! Teferi's Protection: one resolving duration plus a simultaneous phase-out.

use super::*;

fn settle(game: &mut Game) {
    for _ in 0..24 {
        if game.stack.is_empty()
            && game.pending_triggers.is_empty()
            && game.pending_decisions.is_empty()
        {
            return;
        }
        assert!(
            game.pending_decisions.is_empty(),
            "Teferi's Protection asks for no decisions"
        );
        let player = game.priority;
        game.apply(player, Action::PassPriority)
            .expect("priority can be passed");
    }
    panic!("the spell did not settle");
}

#[test]
fn it_locks_life_protects_the_player_phases_their_board_and_exiles_itself() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.step = Step::PrecombatMain;
    game.active_player = PlayerId::One;
    game.priority = PlayerId::One;
    let protected = creature(10_300, cards::SAVANNAH_LIONS, PlayerId::One);
    let protected_id = protected.card.id;
    let opponent = creature(10_301, cards::GRIZZLY_BEARS, PlayerId::Two);
    let opponent_id = opponent.card.id;
    let mut attached = creature(10_302, cards::PACIFISM, PlayerId::Two);
    attached.attached_to = Some(protected_id);
    let attached_id = attached.card.id;
    game.battlefield.extend([protected, opponent, attached]);
    let spell = card(10_303, cards::TEFERIS_PROTECTION, PlayerId::One);
    let spell_id = spell.id;
    game.players[PlayerId::One.index()].hand.push(spell);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == spell_id))
        .expect("three mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == opponent_id),
        "the opponent's unattached permanent stays",
    );
    assert!(
        game.phased_out
            .iter()
            .any(|permanent| permanent.card.id == protected_id),
        "the controller's permanent phases out",
    );
    assert!(
        game.phased_out
            .iter()
            .any(|permanent| permanent.card.id == attached_id),
        "an opponent's Aura attached to it phases out indirectly",
    );
    assert!(
        game.players[PlayerId::One.index()]
            .exile
            .iter()
            .any(|card| card.definition == cards::TEFERIS_PROTECTION),
        "the resolving instant exiles itself",
    );

    let before = game.players[PlayerId::One.index()].life;
    game.gain_life(PlayerId::One, 4);
    game.lose_life(PlayerId::One, 3);
    assert!(!game.can_pay_effect_payment(PlayerId::One, ResolvedEffectPayment::Life(1),));
    game.damage_cannot_be_prevented_this_turn = true;
    let dealt = game.damage_target_from_kind(
        Some(opponent_id),
        Some(Target::Player(PlayerId::One)),
        2,
        false,
    );
    assert_eq!(dealt, 2, "unpreventable damage is still dealt");
    assert_eq!(
        game.players[PlayerId::One.index()].life,
        before,
        "none of gain, loss, payment, or damage changes the locked total",
    );
}

#[test]
fn the_duration_and_indirect_phase_out_end_on_the_controllers_next_turn() {
    let mut game = ready_game();
    game.battlefield.clear();
    let host = creature(10_310, cards::SAVANNAH_LIONS, PlayerId::One);
    let host_id = host.card.id;
    let mut aura = creature(10_311, cards::PACIFISM, PlayerId::Two);
    aura.attached_to = Some(host_id);
    let aura_id = aura.card.id;
    game.battlefield.extend([host, aura]);
    game.phase_out(host_id);

    game.commit_next_turn(PlayerId::Two, Vec::new());
    assert_eq!(
        game.phased_out.len(),
        2,
        "the opponent's turn is still inside the wait"
    );
    game.commit_next_turn(PlayerId::One, Vec::new());

    assert!(game.phased_out.is_empty());
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == host_id)
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == aura_id)
    );
}
