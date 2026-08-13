//! Regeneration as a declarative effect.
//!
//! The shield machinery and the destroy-event replacement already existed and
//! are exercised elsewhere; what is new is an `EffectDef` that arms a shield,
//! so a printed "{cost}: Regenerate this creature" is an ordinary activated
//! ability instead of an engine-level card branch. These tests drive it the
//! way a player would: find the ability in the legal-action list, pay for it,
//! and let the shield meet a real destruction.

use super::*;
use crate::ImplementationStatus;

/// Sedge Troll is the card that used to reach regeneration through a
/// card-identity escape valve, so it is the one that proves the declarative
/// path replaced it rather than joining it.
fn troll_game() -> (Game, GameObjectId) {
    let mut game = ready_game();
    let troll = creature(10_000, cards::SEDGE_TROLL, PlayerId::One);
    let troll_id = troll.card.id;
    game.battlefield.push(troll);
    game.players[PlayerId::One.index()].mana_pool.black = 4;
    (game, troll_id)
}

fn regenerate_actions(game: &Game, source: GameObjectId) -> Vec<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(|action| {
            matches!(action, Action::ActivateAbility { source: actual, .. } if *actual == source)
        })
        .collect()
}

fn arm_shield(game: &mut Game, source: GameObjectId) {
    let actions = regenerate_actions(game, source);
    assert_eq!(
        actions.len(),
        1,
        "the regeneration ability must be offered exactly once, not once per path"
    );
    game.apply(PlayerId::One, actions[0].clone())
        .expect("the regeneration ability activates");
    pass_priority_pair(game);
}

fn troll(game: &Game, id: GameObjectId) -> Option<&Permanent> {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
}

#[test]
fn a_declarative_regeneration_ability_arms_one_shield() {
    let (mut game, troll_id) = troll_game();
    assert_eq!(
        troll(&game, troll_id)
            .expect("the Troll is on the battlefield")
            .regeneration_shields,
        0,
    );

    arm_shield(&mut game, troll_id);

    let shielded = troll(&game, troll_id).expect("regenerating does nothing to the creature yet");
    assert_eq!(shielded.regeneration_shields, 1);
    assert!(
        !shielded.tapped,
        "a shield waits for a destruction rather than tapping now"
    );
}

/// CR 701.15: regeneration replaces destruction with tapping, removing from
/// combat, and removing all damage. The shield is spent doing it.
#[test]
fn an_armed_shield_replaces_lethal_damage_and_is_spent() {
    let (mut game, troll_id) = troll_game();
    arm_shield(&mut game, troll_id);

    {
        let shielded = game
            .battlefield
            .iter_mut()
            .find(|permanent| permanent.card.id == troll_id)
            .expect("the Troll is on the battlefield");
        shielded.damage = 99;
        shielded.attacking = true;
    }
    game.check_state_based_actions();

    let survivor = troll(&game, troll_id).expect("the shield replaced the destruction");
    assert_eq!(survivor.damage, 0, "regeneration removes all damage");
    assert!(survivor.tapped, "regeneration taps the permanent");
    assert!(!survivor.attacking, "regeneration removes it from combat");
    assert_eq!(survivor.regeneration_shields, 0, "the shield was spent");
}

#[test]
fn a_spent_shield_does_not_save_the_creature_twice() {
    let (mut game, troll_id) = troll_game();
    arm_shield(&mut game, troll_id);

    for _ in 0..2 {
        if let Some(permanent) = game
            .battlefield
            .iter_mut()
            .find(|permanent| permanent.card.id == troll_id)
        {
            permanent.damage = 99;
        }
        game.check_state_based_actions();
    }

    assert!(
        troll(&game, troll_id).is_none(),
        "the second lethal damage had no shield left to replace it"
    );
}

/// A shield is a promise about this turn only. Two activations stack, and
/// whatever is left over is discarded rather than carried forward.
#[test]
fn shields_stack_within_a_turn_and_do_not_survive_cleanup() {
    let (mut game, troll_id) = troll_game();
    arm_shield(&mut game, troll_id);
    arm_shield(&mut game, troll_id);
    assert_eq!(
        troll(&game, troll_id)
            .expect("the Troll is on the battlefield")
            .regeneration_shields,
        2,
        "each activation arms its own shield"
    );

    game.finish_cleanup();

    assert_eq!(
        troll(&game, troll_id)
            .expect("the Troll is on the battlefield")
            .regeneration_shields,
        0,
        "unused shields do not carry to the next turn"
    );
}

/// The clause is now ordinary declarative rules text rather than an engine
/// branch, which is what lets the other blocked regeneration cards reuse it.
#[test]
fn the_regeneration_clause_is_declarative_rather_than_a_card_branch() {
    let catalog = poc::catalog().expect("catalog builds");
    let troll = catalog
        .get(cards::SEDGE_TROLL)
        .expect("Sedge Troll is cataloged");
    let clause = troll
        .rules
        .ability_clauses()
        .iter()
        .find(|ability| ability.text == "{B}: Regenerate this creature.")
        .expect("Sedge Troll prints a regeneration clause");
    assert!(
        matches!(
            clause.declarative_effect(),
            Some(EffectDef::Regenerate {
                object: EffectRecipientDef::Source
            })
        ),
        "the clause should carry the declarative effect, not a custom behavior"
    );
    assert!(
        clause.custom_behavior().is_none(),
        "the card-identity escape valve should be gone from this clause"
    );
}

/// The point of the primitive is the cards it unblocks, so one of them is
/// played here rather than merely counted: cast it, activate it, and let the
/// shield meet a destruction.
#[test]
fn a_newly_unblocked_regenerator_casts_activates_and_survives() {
    let mut game = ready_game();
    let troll = card(11_000, cards::UTHDEN_TROLL, PlayerId::One);
    let troll_card_id = troll.id;
    game.players[PlayerId::One.index()].hand.push(troll);
    game.players[PlayerId::One.index()].mana_pool = ManaPool {
        red: 4,
        colorless: 4,
        ..ManaPool::default()
    };

    game.apply(
        PlayerId::One,
        Action::CastSpell {
            card: troll_card_id,
            choices: CastChoices::default(),
            sacrifices: Vec::new(),
        },
    )
    .expect("Uthden Troll is castable");
    pass_priority_pair(&mut game);

    let troll_id = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::UTHDEN_TROLL)
        .expect("the Troll resolved onto the battlefield")
        .card
        .id;

    arm_shield(&mut game, troll_id);
    game.destroy_permanent(troll_id);

    let survivor = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == troll_id)
        .expect("the shield replaced the destruction");
    assert!(survivor.tapped);
    assert_eq!(survivor.regeneration_shields, 0);
}

/// A card is only unblocked if the engine says its rules are fully executable;
/// a definition that still reported partial coverage would be metadata with a
/// card record around it.
#[test]
fn every_newly_unblocked_regenerator_reports_complete_coverage() {
    let catalog = poc::catalog().expect("catalog builds");
    for definition in [
        cards::DRUDGE_SKELETONS,
        cards::WALL_OF_BONE,
        cards::WILL_O_THE_WISP,
        cards::UTHDEN_TROLL,
        cards::WALL_OF_BRAMBLES,
        cards::LIVING_WALL,
        cards::CLAY_STATUE,
        cards::DROWNED,
        cards::GHOST_SHIP,
        cards::DIABOLIC_MACHINE,
        cards::WALKING_DEAD,
    ] {
        let card = catalog.get(definition).expect("the card is cataloged");
        assert_eq!(
            card.rules.implementation_status(),
            ImplementationStatus::Complete,
            "{} should be fully executable",
            card.name,
        );
        assert!(
            card.rules.ability_clauses().iter().any(|ability| {
                matches!(
                    ability.declarative_effect(),
                    Some(EffectDef::Regenerate { .. })
                )
            }),
            "{} should carry the declarative regeneration clause",
            card.name,
        );
    }
}
