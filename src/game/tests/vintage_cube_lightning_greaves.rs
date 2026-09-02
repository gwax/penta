//! Lightning Greaves: haste and shroud for nothing, and the shroud is what
//! makes the boots hard to take off again.

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

/// Equips `source` onto `host`.
fn equip_to(game: &mut Game, source: GameObjectId, host: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source: actual,
                targets,
                ..
            } => {
                *actual == source
                    && targets
                        .iter()
                        .flat_map(crate::casting::TargetSelection::targets)
                        .any(|target| *target == Target::Permanent(host))
            }
            _ => false,
        })
        .expect("equip is offered for that creature");
    game.apply(PlayerId::One, action)
        .expect("the ability activates");
    drain_pending(game);
}

/// Haste is the half the creature notices: a Giant that arrived this turn
/// attacks the moment the Greaves are on it, and stops being able to when
/// they move on.
#[test]
fn the_greaves_hand_their_haste_to_whatever_wears_them() {
    let mut game = ready_game();
    game.battlefield.clear();
    let greaves = game
        .put_onto_battlefield(PlayerId::One, cards::LIGHTNING_GREAVES)
        .expect("cataloged");
    let mut bears = creature(83_000, cards::GRIZZLY_BEARS, PlayerId::One);
    // Arrived this turn, so it is summoning sick without help.
    bears.entered_controller_turn = game.turns_started[PlayerId::One.index()];
    let bears_id = bears.card.id;
    game.battlefield.push(bears);

    let hasty = |game: &Game| {
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == bears_id)
            .is_some_and(|permanent| {
                game.permanent_has_executable_keyword(permanent, KeywordAbility::Haste)
            })
    };

    assert!(!hasty(&game), "an unequipped creature has no haste");
    equip_to(&mut game, greaves, bears_id);
    assert!(hasty(&game), "and an equipped one does");
}

/// "You can't use one Lightning Greaves to allow two new creatures to attack
/// in the same turn." Both moves happen in the main phase, and haste is read
/// as attackers are declared: whichever creature is wearing them then is the
/// only one that may swing.
#[test]
fn one_pair_of_greaves_hastes_one_attacker_a_turn() {
    let mut game = ready_game();
    game.battlefield.clear();
    let greaves = game
        .put_onto_battlefield(PlayerId::One, cards::LIGHTNING_GREAVES)
        .expect("cataloged");
    let mut ids = Vec::new();
    for instance in [83_200, 83_201] {
        let mut arrival = creature(instance, cards::GRIZZLY_BEARS, PlayerId::One);
        arrival.entered_controller_turn = game.turns_started[PlayerId::One.index()];
        ids.push(arrival.card.id);
        game.battlefield.push(arrival);
    }
    drain_pending(&mut game);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    let hasty = |game: &Game, who: GameObjectId| {
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == who)
            .is_some_and(|permanent| {
                game.permanent_has_executable_keyword(permanent, KeywordAbility::Haste)
            })
    };
    assert!(
        !hasty(&game, ids[0]) && !hasty(&game, ids[1]),
        "both arrived this turn and neither has haste yet",
    );

    // Equip is sorcery speed, so both moves are made here, before combat.
    equip_to(&mut game, greaves, ids[0]);
    assert!(hasty(&game, ids[0]) && !hasty(&game, ids[1]));
    equip_to(&mut game, greaves, ids[1]);
    assert!(
        hasty(&game, ids[1]) && !hasty(&game, ids[0]),
        "the haste went with the Greaves rather than staying behind",
    );

    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.priority = PlayerId::One;
    let can_attack = |game: &Game, who: GameObjectId| {
        game.legal_actions(PlayerId::One).iter().any(
            |action| matches!(action, Action::DeclareAttacker { attacker, .. } if *attacker == who),
        )
    };

    assert!(
        can_attack(&game, ids[1]),
        "the one wearing them when attackers are declared may swing",
    );
    assert!(
        !can_attack(&game, ids[0]),
        "and the one that wore them earlier in the turn may not",
    );
}

/// Shroud is the half the opponent notices, and it does not care whose spell
/// it is: the Greaves protect the creature from its own controller too.
#[test]
fn the_greaves_put_their_creature_out_of_reach_of_everyone() {
    let mut game = ready_game();
    game.battlefield.clear();
    let greaves = game
        .put_onto_battlefield(PlayerId::One, cards::LIGHTNING_GREAVES)
        .expect("cataloged");
    let bears = creature(83_010, cards::GRIZZLY_BEARS, PlayerId::One);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    let bolt = card(83_011, cards::LIGHTNING_BOLT, PlayerId::One);
    let bolt_id = bolt.id;
    game.players[0].hand.push(bolt);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);

    let targetable = |game: &Game| {
        game.legal_actions(PlayerId::One).into_iter().any(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if card == bolt_id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(bears_id)))
        })
    };

    assert!(
        targetable(&game),
        "an unequipped creature can be pointed at"
    );
    equip_to(&mut game, greaves, bears_id);
    assert!(
        !targetable(&game),
        "shroud stops its own controller too, which is the cost of the card",
    );
}

/// Equip targets, and shroud stops targeting: the ruling is that a Greaves
/// wearer is out of reach of your other Equipment too, and the Greaves
/// cannot be taken off it -- only moved, and only once there is somewhere
/// else to move them.
#[test]
fn nothing_else_can_equip_the_creature_wearing_the_greaves() {
    let mut game = ready_game();
    game.battlefield.clear();
    let greaves = game
        .put_onto_battlefield(PlayerId::One, cards::LIGHTNING_GREAVES)
        .expect("cataloged");
    let clamp = game
        .put_onto_battlefield(PlayerId::One, cards::SKULLCLAMP)
        .expect("cataloged");
    let bears = creature(83_100, cards::GRIZZLY_BEARS, PlayerId::One);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    drain_pending(&mut game);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 4);

    let equips = |game: &Game, source: GameObjectId, host: GameObjectId| {
        game.legal_actions(PlayerId::One).into_iter().any(|action| {
            matches!(action, Action::ActivateAbility { source: actual, targets, .. }
                if actual == source
                    && targets
                        .iter()
                        .flat_map(crate::casting::TargetSelection::targets)
                        .any(|target| *target == Target::Permanent(host)))
        })
    };
    assert!(
        equips(&game, clamp, bears_id),
        "the Clamp can name a bare creature",
    );

    equip_to(&mut game, greaves, bears_id);

    assert!(
        !equips(&game, clamp, bears_id),
        "and cannot name it once the Greaves are on",
    );
    assert!(
        !equips(&game, greaves, bears_id),
        "nor can the Greaves name it again themselves",
    );

    // Somewhere else to go is what the ruling says is missing.
    let lions = creature(83_101, cards::SAVANNAH_LIONS, PlayerId::One);
    let lions_id = lions.card.id;
    game.battlefield.push(lions);
    assert!(
        equips(&game, greaves, lions_id),
        "a second creature is somewhere for them to move",
    );
    equip_to(&mut game, greaves, lions_id);
    assert!(
        equips(&game, clamp, bears_id),
        "and the Bears are reachable again the moment the Greaves leave",
    );
}

/// Shroud covers the creature, not the boots. The wearer cannot be named by
/// their Bolt, and the Greaves standing beside it can: answering the boots
/// is how you get at the creature, and it takes the haste with it.
#[test]
fn the_greaves_themselves_are_still_a_target() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[1].hand.clear();
    let greaves = game
        .put_onto_battlefield(PlayerId::One, cards::LIGHTNING_GREAVES)
        .expect("cataloged");
    let mut bears = creature(83_400, cards::GRIZZLY_BEARS, PlayerId::One);
    bears.entered_controller_turn = game.turns_started[PlayerId::One.index()];
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    drain_pending(&mut game);
    equip_to(&mut game, greaves, bears_id);

    let bolt = card(83_401, cards::LIGHTNING_BOLT, PlayerId::Two);
    let bolt_id = bolt.id;
    game.players[1].hand.push(bolt);
    let shieldbreaker = card(83_402, cards::EMBERETH_SHIELDBREAKER, PlayerId::Two);
    let shieldbreaker_id = shieldbreaker.id;
    game.players[1].hand.push(shieldbreaker);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Red, 2);
    // Battle Display is a sorcery, so the answer waits for their own turn.
    game.turn += 1;
    game.active_player = PlayerId::Two;
    game.turns_started[PlayerId::Two.index()] += 1;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::Two;

    let names = |game: &Game, spell: CardInstanceId, victim: GameObjectId| {
        game.legal_actions(PlayerId::Two).into_iter().any(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if card == spell
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(victim)))
        })
    };
    assert!(
        !names(&game, bolt_id, bears_id),
        "the creature wearing them is out of reach",
    );

    let shatter = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == shieldbreaker_id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(greaves))
            }
            _ => false,
        })
        .expect("the boots are an artifact like any other, shroud or no");
    game.apply(PlayerId::Two, shatter).expect("it is cast");
    resolve(&mut game);
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == greaves),
        "the boots were destroyed",
    );
    let bare = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == bears_id)
        .expect("the Bears are still there");
    assert!(
        !game.permanent_has_executable_keyword(bare, KeywordAbility::Shroud),
        "and the creature that wore them is targetable again",
    );
    assert!(
        !game.permanent_has_executable_keyword(bare, KeywordAbility::Haste),
        "with the haste gone the same way",
    );
    assert!(
        names(&game, bolt_id, bears_id),
        "so the Bolt they were holding has somewhere to point",
    );
}
