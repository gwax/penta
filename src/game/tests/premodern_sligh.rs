//! The Sligh cards, and the two mechanics they needed.
//!
//! Barbarian Ring's second ability is gated by threshold, which is a
//! restriction on whether the activation is offered at all rather than on
//! what it does. Goblin Vandal's trigger is an optional payment that trades
//! the swing for an artifact.

use super::*;

fn ready() -> Game {
    let mut game = ready_game();
    game.turn = 5;
    game.turns_started[PlayerId::One.index()] = 5;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::One.index()].graveyard.clear();
    game
}

fn settle(game: &mut Game) {
    for _ in 0..16 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .last()
                .map(|option| vec![option.id])
                .unwrap_or_default();
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the decision accepts what it offered");
            continue;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
}

/// The Ring's burn is offered only once seven cards are in your graveyard,
/// and the mana ability is offered either way -- threshold gates one clause,
/// not the card.
fn ring_activations(graveyard: usize) -> (usize, usize) {
    let mut game = ready();
    let ring = creature(10_000, cards::BARBARIAN_RING, PlayerId::One);
    let ring_id = ring.card.id;
    game.battlefield.push(ring);
    for index in 0..graveyard {
        game.players[PlayerId::One.index()].graveyard.push(card(
            30_000 + u32::try_from(index).expect("small"),
            cards::GRIZZLY_BEARS,
            PlayerId::One,
        ));
    }
    game.players[PlayerId::One.index()].mana_pool.red = 1;

    let actions = game.legal_actions(PlayerId::One);
    let burn = actions
        .iter()
        .filter(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == ring_id),
        )
        .count();
    let mana = actions
        .iter()
        .filter(
            |action| matches!(action, Action::ActivateManaAbility { source, .. } if *source == ring_id),
        )
        .count();
    (burn, mana)
}

#[test]
fn barbarian_ring_withholds_its_burn_below_threshold() {
    let (burn, mana) = ring_activations(6);
    assert_eq!(burn, 0, "six cards is one short of threshold");
    assert!(mana > 0, "but it still taps for red");
}

#[test]
fn barbarian_ring_offers_its_burn_at_threshold() {
    let (burn, mana) = ring_activations(7);
    assert!(burn > 0, "seven cards turns the land into a burn spell");
    assert!(mana > 0, "and it still taps for red");
}

/// Only your own graveyard counts: filling the opponent's does nothing.
#[test]
fn barbarian_ring_counts_only_your_own_graveyard() {
    let mut game = ready();
    let ring = creature(10_000, cards::BARBARIAN_RING, PlayerId::One);
    let ring_id = ring.card.id;
    game.battlefield.push(ring);
    for index in 0..9 {
        game.players[PlayerId::Two.index()].graveyard.push(card(
            30_000 + index,
            cards::GRIZZLY_BEARS,
            PlayerId::Two,
        ));
    }
    game.players[PlayerId::One.index()].mana_pool.red = 1;

    let burn = game
        .legal_actions(PlayerId::One)
        .iter()
        .filter(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == ring_id),
        )
        .count();
    assert_eq!(burn, 0, "their graveyard is not your threshold");
}

/// And once it is offered, it actually deals its two.
#[test]
fn barbarian_ring_deals_two_and_sacrifices_itself() {
    let mut game = ready();
    let ring = creature(10_000, cards::BARBARIAN_RING, PlayerId::One);
    let ring_id = ring.card.id;
    game.battlefield.push(ring);
    let target = creature(10_001, cards::SERRA_ANGEL, PlayerId::Two);
    let target_id = target.card.id;
    game.battlefield.push(target);
    for index in 0..7 {
        game.players[PlayerId::One.index()].graveyard.push(card(
            30_000 + index,
            cards::GRIZZLY_BEARS,
            PlayerId::One,
        ));
    }
    game.players[PlayerId::One.index()].mana_pool.red = 1;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } => {
                *source == ring_id
                    && targets
                        .iter()
                        .any(|slot| slot.targets().contains(&Target::Permanent(target_id)))
            }
            _ => false,
        })
        .expect("the Angel can be named");
    game.apply(PlayerId::One, action).expect("it is activated");
    settle(&mut game);

    let damage = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == target_id)
        .map(|permanent| permanent.damage);
    assert_eq!(damage, Some(2), "two damage to the Angel");
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == ring_id),
        "and the land sacrificed itself to do it",
    );
}

/// Connecting with the Vandal offers a trade: pay {R} to break an artifact
/// and deal no combat damage this turn.
#[test]
fn the_vandal_trades_its_damage_for_an_artifact() {
    let mut game = ready();
    let vandal = creature(10_000, cards::GOBLIN_VANDAL, PlayerId::One);
    let vandal_id = vandal.card.id;
    game.battlefield.push(vandal);
    let scroll = creature(10_001, cards::BLACK_VISE, PlayerId::Two);
    let scroll_id = scroll.card.id;
    game.battlefield.push(scroll);
    game.players[PlayerId::One.index()].mana_pool.red = 1;

    let attacking = game.trigger_event_object(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == vandal_id)
            .expect("still there"),
    );
    game.capture_battlefield_triggers(&CommittedTriggerEvent::AttacksAndIsNotBlocked {
        object: attacking,
    });
    settle(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == scroll_id),
        "the artifact was destroyed",
    );
    let assigns_none = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == vandal_id)
        .is_some_and(|permanent| {
            game.has_applied_rule(permanent, AppliedRuleDef::AssignsNoCombatDamage)
        });
    assert!(assigns_none, "and the Vandal deals nothing this turn");
}
