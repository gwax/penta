//! Animate Dead: two mana for anything that died, a point of power off it,
//! and the creature going away again with the Aura.

use super::*;

fn settle(game: &mut Game) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = if decision.minimum > 1 {
                decision
                    .options
                    .iter()
                    .map(|option| option.id)
                    .take(decision.minimum)
                    .collect()
            } else {
                decision
                    .options
                    .iter()
                    .find(|option| option.label != "Decline")
                    .or_else(|| decision.options.first())
                    .map(|option| vec![option.id])
                    .unwrap_or_default()
            };
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
    game.check_state_based_actions();
}

/// Animate Dead in hand with the mana for it, and `theirs` in the opponent's
/// graveyard.
fn staged(theirs: &[CardDefinitionId]) -> (Game, CardInstanceId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    game.players[1].graveyard.clear();
    for (index, definition) in theirs.iter().enumerate() {
        game.players[1].graveyard.push(card(
            89_000 + u32::try_from(index).expect("a small graveyard"),
            *definition,
            PlayerId::Two,
        ));
    }
    let animate = card(89_500, cards::ANIMATE_DEAD, PlayerId::One);
    let animate_id = animate.id;
    game.players[0].hand.push(animate);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    game.turns_started = [2, 1];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, animate_id)
}

fn cast(game: &mut Game, animate: CardInstanceId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == animate))
        .expect("two mana casts it");
    game.apply(PlayerId::One, action).expect("it is cast");
    settle(game);
    drain_pending(game);
}

fn permanent(game: &Game, definition: CardDefinitionId) -> Option<&Permanent> {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == definition)
}

/// It brings the creature back under your control with the Aura on it.
#[test]
fn it_reanimates_under_your_control_and_attaches() {
    let (mut game, animate) = staged(&[cards::SERRA_ANGEL]);

    cast(&mut game, animate);

    let angel = permanent(&game, cards::SERRA_ANGEL).expect("the Angel is back");
    assert_eq!(angel.controller, PlayerId::One, "under your control");
    let angel_id = angel.card.id;
    let aura = permanent(&game, cards::ANIMATE_DEAD).expect("the Aura stayed");
    assert_eq!(
        aura.attached_to,
        Some(angel_id),
        "attached to what it brought back",
    );
}

/// The price: a point of power off whatever came back.
#[test]
fn the_reanimated_creature_is_a_point_weaker() {
    let (mut game, animate) = staged(&[cards::SERRA_ANGEL]);

    cast(&mut game, animate);

    let angel = permanent(&game, cards::SERRA_ANGEL).expect("the Angel is back");
    assert_eq!(game.power(angel), Some(3), "a 4/4 comes back a 3/4");
    assert_eq!(game.toughness(angel), Some(4), "and keeps its toughness");
}

/// The Aura leaving takes the creature with it, back to its owner's
/// graveyard.
#[test]
fn the_creature_is_sacrificed_when_the_aura_leaves() {
    let (mut game, animate) = staged(&[cards::SERRA_ANGEL]);
    cast(&mut game, animate);
    let aura = permanent(&game, cards::ANIMATE_DEAD)
        .expect("it is there")
        .card
        .id;

    game.move_permanents_to_graveyard(&[aura]);
    settle(&mut game);
    drain_pending(&mut game);

    assert!(
        permanent(&game, cards::SERRA_ANGEL).is_none(),
        "the creature went with it",
    );
    assert!(
        game.players[1]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::SERRA_ANGEL),
        "back to its owner's graveyard",
    );
}

/// "Enchant creature card in a graveyard" is the spell's own target, so a
/// graveyard with nothing in it worth naming is a spell that cannot be cast
/// at all.
#[test]
fn it_cannot_be_cast_without_a_creature_card() {
    for graveyard in [Vec::new(), vec![cards::LIGHTNING_BOLT]] {
        let (game, animate) = staged(&graveyard);

        assert!(
            !game
                .legal_actions(PlayerId::One)
                .iter()
                .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == animate)),
            "two mana and no creature card is no cast",
        );
    }
}

/// The card is the spell's target, so answering it in response counters the
/// Aura on resolution rather than leaving it on the battlefield enchanting
/// nothing.
#[test]
fn a_card_that_leaves_in_response_counters_the_spell() {
    let (mut game, animate) = staged(&[cards::SERRA_ANGEL]);
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == animate))
        .expect("two mana casts it");
    game.apply(PlayerId::One, action).expect("it is cast");

    // The Angel leaves the graveyard while the Aura is still on the stack.
    let angel = game.players[1]
        .graveyard
        .pop()
        .expect("the Angel was the only card there");
    game.players[1].exile.push(angel);
    settle(&mut game);

    assert!(
        permanent(&game, cards::ANIMATE_DEAD).is_none(),
        "the Aura was countered rather than resolving",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::ANIMATE_DEAD),
        "and went to its owner's graveyard",
    );
}
