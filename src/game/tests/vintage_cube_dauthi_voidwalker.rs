//! Dauthi Voidwalker: a body nothing ordinary can block, an opponent's
//! graveyard that never fills, and one card off that pile for free.

use super::*;

/// The Voidwalker on the battlefield since last turn, with `theirs` under
/// Player Two.
fn staged(theirs: &[CardDefinitionId]) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[1].hand.clear();
    game.players[0].graveyard.clear();
    game.players[1].graveyard.clear();
    let mut ids = Vec::new();
    for definition in theirs {
        ids.push(
            game.put_onto_battlefield(PlayerId::Two, *definition)
                .expect("cataloged"),
        );
    }
    let voidwalker = game
        .put_onto_battlefield(PlayerId::One, cards::DAUTHI_VOIDWALKER)
        .expect("cataloged");
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [2, 1];
    drain_pending(&mut game);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, voidwalker, ids)
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
            .expect("the offered choice is legal");
            continue;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    game.check_state_based_actions();
}

fn exiled(game: &Game, player: PlayerId, definition: CardDefinitionId) -> Option<&CardInstance> {
    game.players[player.index()]
        .exile
        .iter()
        .find(|card| card.definition == definition)
}

fn in_graveyard(game: &Game, player: PlayerId, definition: CardDefinitionId) -> bool {
    game.players[player.index()]
        .graveyard
        .iter()
        .any(|card| card.definition == definition)
}

/// A creature of theirs that dies is exiled with a void counter instead.
#[test]
fn their_dying_creature_is_exiled_with_a_counter() {
    let (mut game, _voidwalker, ids) = staged(&[cards::GRIZZLY_BEARS]);

    game.move_permanents_to_graveyard(&[ids[0]]);
    settle(&mut game);

    assert!(
        !in_graveyard(&game, PlayerId::Two, cards::GRIZZLY_BEARS),
        "nothing of theirs reaches a graveyard",
    );
    let card = exiled(&game, PlayerId::Two, cards::GRIZZLY_BEARS).expect("it is exiled instead");
    assert_eq!(
        card.counters(CounterKind::named("void")),
        1,
        "with a void counter"
    );
}

/// "From anywhere" is the whole card: a discard is exiled the same way.
#[test]
fn a_card_they_discard_is_exiled_too() {
    let (mut game, _voidwalker, _) = staged(&[]);
    let discarded = game
        .build_zone(PlayerId::Two, &[cards::LIGHTNING_BOLT])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let discarded_id = discarded.id;
    game.players[1].hand.push(discarded);

    game.discard_cards(PlayerId::Two, &[discarded_id]);
    settle(&mut game);

    assert!(
        !in_graveyard(&game, PlayerId::Two, cards::LIGHTNING_BOLT),
        "the discard is exiled",
    );
    let card = exiled(&game, PlayerId::Two, cards::LIGHTNING_BOLT).expect("it is exiled");
    assert_eq!(
        card.counters(CounterKind::named("void")),
        1,
        "with a void counter"
    );
}

/// Your own graveyard is untouched: the clause names an opponent's.
#[test]
fn your_own_cards_still_reach_your_graveyard() {
    let (mut game, _voidwalker, _) = staged(&[]);
    let bears = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);

    game.move_permanents_to_graveyard(&[bears]);
    settle(&mut game);

    assert!(
        in_graveyard(&game, PlayerId::One, cards::GRIZZLY_BEARS),
        "yours dies the ordinary way",
    );
}

/// Sacrificing it plays one of the cards it took, for nothing.
#[test]
fn sacrificing_it_plays_a_marked_card_for_free() {
    let (mut game, voidwalker, ids) = staged(&[cards::GRIZZLY_BEARS]);
    game.move_permanents_to_graveyard(&[ids[0]]);
    settle(&mut game);
    let marked = exiled(&game, PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("it is exiled")
        .id;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == voidwalker),
        )
        .expect("tap and sacrifice is available");
    game.apply(PlayerId::One, action).expect("it activates");
    // Deliberately not settled: the offer to cast is a standing decision,
    // and casting is what takes it away. Answering it would be the decline.
    for _ in 0..8 {
        if game.pending_decisions.is_empty() && !game.stack.is_empty() {
            let player = game.priority;
            if game.apply(player, Action::PassPriority).is_err() {
                break;
            }
            continue;
        }
        break;
    }

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != voidwalker),
        "it sacrificed itself to pay",
    );
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == marked))
        .expect("their card is castable with no mana at all");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game);

    assert!(
        game.battlefield.iter().any(
            |permanent| permanent.card.definition == cards::GRIZZLY_BEARS
                && permanent.controller == PlayerId::One
        ),
        "and it arrives under your control",
    );
}

/// Shadow: an ordinary creature cannot be declared against it, and another
/// shadow creature can.
#[test]
fn only_shadow_blocks_shadow() {
    let (mut game, voidwalker, ids) = staged(&[cards::GRIZZLY_BEARS]);
    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::DAUTHI_VOIDWALKER)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        if permanent.card.id == voidwalker {
            permanent.attacking = true;
        }
    }
    game.step = Step::DeclareBlockers;
    game.blockers_declared = false;

    let blockers = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .filter_map(|action| match action {
            Action::DeclareBlocker { blocker, attacker } if attacker == voidwalker => Some(blocker),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(
        !blockers.contains(&ids[0]),
        "the Bears cannot be declared against shadow",
    );
    assert!(
        blockers.contains(&theirs),
        "their own Voidwalker can, which is the whole of the exception",
    );
}

/// The same keyword read from the other side: it cannot block an ordinary
/// attacker either.
#[test]
fn it_cannot_block_an_ordinary_attacker() {
    let (mut game, voidwalker, ids) = staged(&[cards::GRIZZLY_BEARS]);
    for permanent in &mut game.battlefield {
        if permanent.card.id == ids[0] {
            permanent.attacking = true;
        }
    }
    game.active_player = PlayerId::Two;
    game.step = Step::DeclareBlockers;
    game.blockers_declared = false;

    assert!(
        game.legal_actions(PlayerId::One)
            .iter()
            .all(|action| !matches!(
                action,
                Action::DeclareBlocker { blocker, .. } if *blocker == voidwalker
            )),
        "shadow blocks only shadow",
    );
}

/// "Tokens still die while Dauthi Voidwalker is on the battlefield." What
/// the replacement names is a card, and a token is not one.
#[test]
fn a_token_of_theirs_still_dies() {
    let (mut game, _voidwalker, _ids) = staged(&[]);
    let token = token_permanent(
        95_400,
        tokens::creature(&["Bird"], &[ManaColor::White], 1, 1),
        PlayerId::Two,
    );
    let token_id = token.card.id;
    game.battlefield.push(token);
    let exiled_before = game.players[1].exile.len();

    game.move_permanents_to_graveyard(&[token_id]);
    settle(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == token_id),
        "it left the battlefield",
    );
    assert_eq!(
        game.players[1].exile.len(),
        exiled_before,
        "and nothing of theirs was exiled with a counter",
    );
}

/// "Abilities that would trigger when those creatures die won't trigger."
/// A Messenger Drake that is exiled instead of dying draws its controller
/// nothing.
#[test]
fn a_death_trigger_of_theirs_never_fires() {
    let (mut game, _voidwalker, ids) = staged(&[cards::MESSENGER_DRAKE]);
    let held = game.players[1].hand.len();

    game.move_permanents_to_graveyard(&[ids[0]]);
    settle(&mut game);

    assert!(
        exiled(&game, PlayerId::Two, cards::MESSENGER_DRAKE).is_some(),
        "it was exiled rather than buried",
    );
    assert!(
        !in_graveyard(&game, PlayerId::Two, cards::MESSENGER_DRAKE),
        "so it never reached the graveyard",
    );
    assert_eq!(
        game.players[1].hand.len(),
        held,
        "and a creature that did not die draws nobody a card",
    );
}
