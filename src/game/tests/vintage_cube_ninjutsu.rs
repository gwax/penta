//! Fallen Shinobi: ninjutsu, and playing somebody else's cards for free.

use super::*;

/// Puts `attacker` into the window ninjutsu lives in: blockers declared and
/// none of them on it, attacking player two.
fn attacking_unblocked(game: &mut Game, attacker: GameObjectId) {
    game.step = Step::DeclareBlockers;
    game.attackers_declared = true;
    game.blockers_declared = true;
    game.priority = PlayerId::One;
    for permanent in &mut game.battlefield {
        if permanent.card.id == attacker {
            permanent.attacking = true;
            permanent.tapped = true;
            permanent.attack_defender = Some(AttackDefender::Player(PlayerId::Two));
        }
    }
}

/// The one ninjutsu activation on offer for the card in hand.
fn ninjutsu_action(game: &Game, ninja: GameObjectId) -> Option<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == ninja))
}

/// Ninjutsu swaps the unblocked attacker for the ninja, which arrives tapped
/// and attacking the same player.
#[test]
fn ninjutsu_swaps_an_unblocked_attacker_for_the_ninja() {
    let mut game = ready_game();
    game.battlefield.clear();
    let bears = creature(90_000, cards::GRIZZLY_BEARS, PlayerId::One);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    let shinobi = card(90_001, cards::FALLEN_SHINOBI, PlayerId::One);
    let shinobi_id = shinobi.id;
    game.players[0].hand.push(shinobi);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);
    attacking_unblocked(&mut game, bears_id);

    let action = ninjutsu_action(&game, shinobi_id).expect("ninjutsu is offered");
    game.apply(PlayerId::One, action).expect("it is activated");
    pass_until_decision(&mut game);
    drain_pending(&mut game);

    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS),
        "the attacker went back to hand",
    );
    let ninja = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::FALLEN_SHINOBI)
        .expect("the ninja arrived");
    assert!(ninja.tapped, "tapped");
    assert!(ninja.attacking, "and attacking");
    assert_eq!(
        ninja.attack_defender,
        Some(AttackDefender::Player(PlayerId::Two)),
        "the same player the returned creature was attacking",
    );
}

/// "The ninjutsu ability can be activated only after blockers have been
/// declared. Before then, attacking creatures are neither blocked nor
/// unblocked." So the window opens with the blocker declaration rather than
/// the attack one, and it stays open through the end of combat -- where the
/// swap is still legal and simply too late to deal any damage.
#[test]
fn ninjutsu_waits_for_blockers_and_lasts_to_the_end_of_combat() {
    let mut game = ready_game();
    game.battlefield.clear();
    let bears = creature(90_010, cards::GRIZZLY_BEARS, PlayerId::One);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    let shinobi = card(90_011, cards::FALLEN_SHINOBI, PlayerId::One);
    let shinobi_id = shinobi.id;
    game.players[0].hand.push(shinobi);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);

    assert!(
        ninjutsu_action(&game, shinobi_id).is_none(),
        "no attackers yet, so nothing to return",
    );

    game.step = Step::DeclareAttackers;
    game.attackers_declared = true;
    game.priority = PlayerId::One;
    for permanent in &mut game.battlefield {
        if permanent.card.id == bears_id {
            permanent.attacking = true;
            permanent.tapped = true;
            permanent.attack_defender = Some(AttackDefender::Player(PlayerId::Two));
        }
    }
    assert!(
        ninjutsu_action(&game, shinobi_id).is_none(),
        "and an attacker nobody has had the chance to block is neither blocked nor unblocked",
    );

    game.step = Step::DeclareBlockers;
    game.blockers_declared = true;
    assert!(
        ninjutsu_action(&game, shinobi_id).is_some(),
        "the window opens once the blocks are in",
    );

    for step in [Step::CombatDamage, Step::EndOfCombat] {
        game.step = step;
        assert!(
            ninjutsu_action(&game, shinobi_id).is_some(),
            "{step:?} is still inside it",
        );
    }

    game.step = Step::PostcombatMain;
    assert!(
        ninjutsu_action(&game, shinobi_id).is_none(),
        "and combat is where it ends",
    );
}

/// Connecting exiles the top two of the defending player's library and lets
/// their attacker play them, free, this turn.
#[test]
fn the_shinobi_plays_the_two_cards_it_took_for_free() {
    let mut game = ready_game();
    game.battlefield.clear();
    let shinobi = creature(90_020, cards::FALLEN_SHINOBI, PlayerId::One);
    let shinobi_id = shinobi.card.id;
    game.battlefield.push(shinobi);
    game.players[1].library.clear();
    // Serra Angel costs five; the point is that none of it is paid.
    game.players[1]
        .library
        .push(card(90_021, cards::GRIZZLY_BEARS, PlayerId::Two));
    game.players[1]
        .library
        .push(card(90_022, cards::SERRA_ANGEL, PlayerId::Two));

    game.damage_target_from_kind(
        Some(shinobi_id),
        Some(Target::Player(PlayerId::Two)),
        5,
        true,
    );
    pass_until_decision(&mut game);
    drain_pending(&mut game);

    assert_eq!(
        game.players[1].exile.len(),
        2,
        "two cards left their library for exile",
    );
    let angel = game.players[1]
        .exile
        .iter()
        .find(|card| card.definition == cards::SERRA_ANGEL)
        .expect("the Angel is one of them");
    let angel_id = angel.id;

    // No mana at all, and it is still castable.
    assert!(
        game.legal_actions(PlayerId::One)
            .into_iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if card == angel_id)),
        "a five-mana creature is free from exile",
    );
    assert!(
        game.legal_actions(PlayerId::Two)
            .into_iter()
            .all(|action| !matches!(action, Action::CastSpell { card, .. } if card == angel_id)),
        "and its owner may not play it",
    );
}

/// The permission is for this turn only.
#[test]
fn the_free_plays_lapse_at_end_of_turn() {
    let mut game = ready_game();
    game.battlefield.clear();
    let shinobi = creature(90_030, cards::FALLEN_SHINOBI, PlayerId::One);
    let shinobi_id = shinobi.card.id;
    game.battlefield.push(shinobi);
    game.players[1].library.clear();
    game.players[1]
        .library
        .push(card(90_031, cards::GRIZZLY_BEARS, PlayerId::Two));

    game.damage_target_from_kind(
        Some(shinobi_id),
        Some(Target::Player(PlayerId::Two)),
        5,
        true,
    );
    pass_until_decision(&mut game);
    drain_pending(&mut game);
    let taken = game.players[1].exile.first().expect("one card taken").id;
    assert!(
        game.legal_actions(PlayerId::One)
            .into_iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if card == taken)),
        "playable while the turn lasts",
    );

    game.active_player = PlayerId::Two;
    assert!(
        game.legal_actions(PlayerId::One)
            .into_iter()
            .all(|action| !matches!(action, Action::CastSpell { card, .. } if card == taken)),
        "and gone once the turn is over",
    );
}

/// "The triggered ability doesn't change when you can play the exiled cards.
/// If a land card is exiled, you can play it only during your main phase and
/// only if you have an available land play remaining."
#[test]
fn a_land_it_took_still_wants_a_land_drop() {
    let mut game = ready_game();
    game.battlefield.clear();
    let shinobi = creature(90_040, cards::FALLEN_SHINOBI, PlayerId::One);
    let shinobi_id = shinobi.card.id;
    game.battlefield.push(shinobi);
    game.players[1].library.clear();
    game.players[1]
        .library
        .push(card(90_041, cards::MOUNTAIN, PlayerId::Two));

    game.damage_target_from_kind(
        Some(shinobi_id),
        Some(Target::Player(PlayerId::Two)),
        5,
        true,
    );
    pass_until_decision(&mut game);
    drain_pending(&mut game);
    let taken = game.players[1].exile.first().expect("one card taken").id;

    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.players[0].lands_played_this_turn = 1;
    assert!(
        game.legal_actions(PlayerId::One)
            .into_iter()
            .all(|action| !matches!(action, Action::PlayLand { card, .. } if card == taken)),
        "the land drop was already spent",
    );

    game.players[0].lands_played_this_turn = 0;
    assert!(
        game.legal_actions(PlayerId::One)
            .into_iter()
            .any(|action| matches!(action, Action::PlayLand { card, .. } if card == taken)),
        "and with one available their Mountain is yours to play",
    );
}

/// "Casting an exiled card causes it to leave exile. You can't cast it
/// multiple times."
#[test]
fn a_card_it_took_may_only_be_played_once() {
    let mut game = ready_game();
    game.battlefield.clear();
    let shinobi = creature(90_050, cards::FALLEN_SHINOBI, PlayerId::One);
    let shinobi_id = shinobi.card.id;
    game.battlefield.push(shinobi);
    game.players[1].library.clear();
    game.players[1]
        .library
        .push(card(90_051, cards::GRIZZLY_BEARS, PlayerId::Two));

    game.damage_target_from_kind(
        Some(shinobi_id),
        Some(Target::Player(PlayerId::Two)),
        5,
        true,
    );
    pass_until_decision(&mut game);
    drain_pending(&mut game);
    let taken = game.players[1].exile.first().expect("one card taken").id;

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == taken))
        .expect("their bear is free to cast");
    game.apply(PlayerId::One, cast).expect("it is cast");
    pass_until_decision(&mut game);
    drain_pending(&mut game);

    assert!(
        game.battlefield.iter().any(
            |permanent| permanent.card.definition == cards::GRIZZLY_BEARS
                && permanent.controller == PlayerId::One
        ),
        "it arrived under the Shinobi's controller",
    );
    assert!(
        game.players[1].exile.is_empty(),
        "casting it took it out of exile",
    );
    assert!(
        game.legal_actions(PlayerId::One)
            .into_iter()
            .all(|action| !matches!(action, Action::CastSpell { card, .. } if card == taken)),
        "and there is nothing left to cast a second time",
    );
}
