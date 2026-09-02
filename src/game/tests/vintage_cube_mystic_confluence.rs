//! Mystic Confluence: three modes from a list of three, and the same one may
//! be taken more than once.

use super::*;

/// Player One holding a Confluence with the mana to cast it.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    let confluence = game
        .build_zone(PlayerId::One, &[cards::MYSTIC_CONFLUENCE])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = confluence.id;
    game.players[0].hand.push(confluence);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 5);
    game.priority = PlayerId::One;
    (game, id)
}

fn casts(game: &Game, card: GameObjectId) -> Vec<(Vec<ModeId>, Vec<Target>)> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell {
                card: id, choices, ..
            } if id == card => Some((
                choices.modes().to_vec(),
                choices.iter_targets().copied().collect(),
            )),
            _ => None,
        })
        .collect()
}

fn cast_with(game: &mut Game, card: GameObjectId, wanted: &[ModeId], targets: &[Target]) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell {
                card: id, choices, ..
            } => {
                *id == card
                    && choices.modes() == wanted
                    && choices.iter_targets().copied().collect::<Vec<_>>() == targets
            }
            _ => false,
        })
        .expect("that combination of modes and targets is on offer");
    game.apply(PlayerId::One, action).expect("it is castable");
}

fn settle(game: &mut Game) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .take(decision.minimum.max(1))
                .map(|option| option.id)
                .collect();
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
            return;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            return;
        }
    }
}

/// The same, except that a payment offered is a payment taken. "Unless its
/// controller pays" is only a counter when they cannot or will not.
fn settle_paying(game: &mut Game) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .find(|option| option.label.starts_with("Pay "))
                .map_or_else(
                    || {
                        decision
                            .options
                            .iter()
                            .take(decision.minimum.max(1))
                            .map(|option| option.id)
                            .collect()
                    },
                    |option| vec![option.id],
                );
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
            return;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            return;
        }
    }
}

/// Three draws is one of the choices, and it is three cards.
#[test]
fn three_draws_is_three_cards() {
    let (mut game, confluence) = staged();
    let draw = ModeId::from_index(2).expect("the third mode");
    let before = game.players[0].hand.len();

    cast_with(&mut game, confluence, &[draw, draw, draw], &[]);
    settle(&mut game);

    assert_eq!(
        game.players[0].hand.len(),
        before - 1 + 3,
        "the Confluence left hand and three cards came back",
    );
}

/// Two bounces means two target slots, so two different creatures may be
/// picked up at once.
#[test]
fn the_same_mode_twice_gets_two_targets() {
    let (mut game, confluence) = staged();
    let first = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    let second = game
        .put_onto_battlefield(PlayerId::Two, cards::SERRA_ANGEL)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    let bounce = ModeId::from_index(1).expect("the second mode");
    let draw = ModeId::from_index(2).expect("the third mode");
    cast_with(
        &mut game,
        confluence,
        &[bounce, bounce, draw],
        &[Target::Permanent(first), Target::Permanent(second)],
    );
    settle(&mut game);

    assert!(
        game.battlefield.is_empty(),
        "both creatures went back to hand",
    );
    assert_eq!(game.players[1].hand.len(), 2);
}

/// Every offered selection takes exactly three modes.
#[test]
fn it_always_chooses_exactly_three() {
    let (game, confluence) = staged();
    let offered = casts(&game, confluence);

    assert!(!offered.is_empty(), "it is castable");
    assert!(
        offered.iter().all(|(modes, _)| modes.len() == 3),
        "choose three, never two and never four",
    );
}

/// With nothing on the stack and no creature out, the counter and bounce
/// modes have nothing to point at, so only the draw is on offer.
#[test]
fn a_mode_with_no_legal_target_is_not_offered() {
    let (game, confluence) = staged();
    let draw = ModeId::from_index(2).expect("the third mode");
    let offered = casts(&game, confluence);

    assert_eq!(
        offered.len(),
        1,
        "an empty board leaves one way to choose three",
    );
    assert_eq!(offered[0].0, vec![draw, draw, draw]);
}

/// The counter mode is a soft one: the spell's controller may buy it back for
/// three, and with the mana to spare they do.
#[test]
fn the_counter_mode_can_be_paid_through() {
    for (available, expected_life) in [(0, 20), (3, 17)] {
        let (mut game, confluence) = staged();
        let bolt = game
            .build_zone(PlayerId::Two, &[cards::LIGHTNING_BOLT])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        let bolt_id = bolt.id;
        game.players[1].hand.push(bolt);
        game.add_unrestricted_mana(PlayerId::Two, ManaColor::Red, 1 + available);
        game.priority = PlayerId::Two;
        // Pointed across the table, so a Bolt that resolves is visible in a
        // life total and a Bolt that was countered is not. Both end up in the
        // same graveyard, so the graveyard cannot tell them apart.
        let cast = game
            .legal_actions(PlayerId::Two)
            .into_iter()
            .find(|action| match action {
                Action::CastSpell { card, choices, .. } => {
                    *card == bolt_id
                        && choices
                            .iter_targets()
                            .any(|target| *target == Target::Player(PlayerId::One))
                }
                _ => false,
            })
            .expect("the Bolt is castable at you");
        game.apply(PlayerId::Two, cast)
            .expect("it goes on the stack");
        let on_stack = game.stack.last().expect("the Bolt is there").id;
        game.priority = PlayerId::One;

        let counter = ModeId::from_index(0).expect("the first mode");
        let draw = ModeId::from_index(2).expect("the third mode");
        cast_with(
            &mut game,
            confluence,
            &[counter, draw, draw],
            &[Target::Spell(on_stack)],
        );
        settle_paying(&mut game);

        assert_eq!(
            game.players[0].life, expected_life,
            "with {available} spare mana the Bolt should have been paid for",
        );
    }
}

/// "If you choose the first and/or second modes but all of the targets
/// become illegal before it resolves, the spell won't resolve. If you also
/// chose the last mode, you won't draw any cards." The draws are not their
/// own spell; they go down with the rest.
#[test]
fn losing_every_target_takes_the_draws_with_it() {
    let (mut game, confluence) = staged();
    let bears = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;
    let held = game.players[0].hand.len();
    let library = game.players[0].library.len();

    let bounce = ModeId::from_index(1).expect("the second mode");
    let draw = ModeId::from_index(2).expect("the third mode");
    cast_with(
        &mut game,
        confluence,
        &[bounce, draw, draw],
        &[Target::Permanent(bears)],
    );
    game.move_permanents_to_graveyard(&[bears]);
    game.check_state_based_actions();
    settle(&mut game);

    assert_eq!(
        game.players[0].hand.len(),
        held - 1,
        "the Confluence left the hand and nothing came back",
    );
    assert_eq!(
        game.players[0].library.len(),
        library,
        "the draws never happened",
    );
}

/// "If at least one target is still legal, the spell will resolve but will
/// have no effect on any illegal targets." One of two creatures leaving is
/// not the whole of the targets.
#[test]
fn one_target_surviving_carries_the_rest_of_the_spell() {
    let (mut game, confluence) = staged();
    let first = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    let second = game
        .put_onto_battlefield(PlayerId::Two, cards::SERRA_ANGEL)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;
    let held = game.players[0].hand.len();

    let bounce = ModeId::from_index(1).expect("the second mode");
    let draw = ModeId::from_index(2).expect("the third mode");
    cast_with(
        &mut game,
        confluence,
        &[bounce, bounce, draw],
        &[Target::Permanent(first), Target::Permanent(second)],
    );
    game.move_permanents_to_graveyard(&[first]);
    game.check_state_based_actions();
    settle(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == second),
        "the creature that was still there went back to hand",
    );
    assert_eq!(
        game.players[0].hand.len(),
        held,
        "and the draw happened: one card out for the Confluence, one in",
    );
}

/// Two bounces may name one creature. The first return takes it, and the
/// second finds a card in hand rather than the permanent it named, so it
/// does nothing -- but the spell still resolves, and the draw that came with
/// it still happens.
#[test]
fn two_bounces_may_name_the_same_creature() {
    let (mut game, confluence) = staged();
    let bears = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;
    let hand = game.players[0].hand.len();

    let bounce = ModeId::from_index(1).expect("the second mode");
    let draw = ModeId::from_index(2).expect("the third mode");
    cast_with(
        &mut game,
        confluence,
        &[bounce, bounce, draw],
        &[Target::Permanent(bears), Target::Permanent(bears)],
    );
    settle(&mut game);

    assert!(game.battlefield.is_empty(), "the Bears went back");
    assert_eq!(
        game.players[1].hand.len(),
        1,
        "once, not twice: there is only one of them",
    );
    assert_eq!(
        game.players[0].hand.len(),
        hand,
        "and the draw made up for the Confluence leaving your hand",
    );
}
