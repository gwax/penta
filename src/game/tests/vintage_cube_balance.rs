//! Balance as it is cast: two mana that levels the board three ways.
//!
//! Its corners -- the recount between phases, the trigger batching, the
//! shrouded creature it may still take -- are checked where the resolver
//! lives. What this adds is the whole card, cast from a hand and paid for,
//! against a board that is ahead in every category at once.

use super::*;

/// Player One holding a Balance and the mana for it, behind `mine` and
/// against `theirs`.
fn staged(
    mine: &[CardDefinitionId],
    theirs: &[CardDefinitionId],
    hands: (usize, usize),
) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::Two.index()].hand.clear();
    for (index, definition) in mine.iter().enumerate() {
        game.battlefield.push(creature(
            99_900 + u32::try_from(index).expect("a small board"),
            *definition,
            PlayerId::One,
        ));
    }
    for (index, definition) in theirs.iter().enumerate() {
        game.battlefield.push(creature(
            99_950 + u32::try_from(index).expect("a small board"),
            *definition,
            PlayerId::Two,
        ));
    }
    for (player, count) in [(PlayerId::One, hands.0), (PlayerId::Two, hands.1)] {
        for index in 0..count {
            game.players[player.index()].hand.push(card(
                100_000
                    + u32::try_from(index).expect("a small hand")
                    + 20 * u32::from(player == PlayerId::Two),
                cards::LIGHTNING_BOLT,
                player,
            ));
        }
    }
    let balance = card(100_100, cards::BALANCE, PlayerId::One);
    let balance_id = balance.id;
    game.players[PlayerId::One.index()].hand.push(balance);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, balance_id)
}

/// Casts it and answers every choice it asks, giving up whatever is offered
/// first.
fn cast_and_settle(game: &mut Game, balance: GameObjectId) {
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == balance))
        .expect("two mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options: decision
                        .options
                        .iter()
                        .take(decision.minimum)
                        .map(|option| option.id)
                        .collect(),
                },
            )
            .expect("giving up what it asks for is legal");
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

fn lands(game: &Game, player: PlayerId) -> usize {
    game.battlefield
        .iter()
        .filter(|permanent| {
            permanent.controller == player
                && game
                    .permanent_types(permanent)
                    .is_some_and(|types| types.contains(CardType::Land))
        })
        .count()
}

fn creatures(game: &Game, player: PlayerId) -> usize {
    game.battlefield
        .iter()
        .filter(|permanent| {
            permanent.controller == player
                && game
                    .permanent_types(permanent)
                    .is_some_and(CardTypeSet::is_creature)
        })
        .count()
}

/// The card the deck plays it for: behind on lands, on cards and on bodies,
/// and level on all three afterwards.
#[test]
fn it_levels_lands_hands_and_creatures_at_once() {
    let (mut game, balance) = staged(
        &[cards::PLAINS],
        &[
            cards::ISLAND,
            cards::ISLAND,
            cards::ISLAND,
            cards::GRIZZLY_BEARS,
            cards::SERRA_ANGEL,
        ],
        (2, 4),
    );

    cast_and_settle(&mut game, balance);

    assert_eq!(
        (lands(&game, PlayerId::One), lands(&game, PlayerId::Two)),
        (1, 1),
        "their three lands come down to your one",
    );
    assert_eq!(
        (
            creatures(&game, PlayerId::One),
            creatures(&game, PlayerId::Two)
        ),
        (0, 0),
        "and both of their creatures go, because you have none",
    );
    assert_eq!(
        (
            game.players[PlayerId::One.index()].hand.len(),
            game.players[PlayerId::Two.index()].hand.len()
        ),
        (2, 2),
        "the hands meet at the smaller of the two",
    );
}

/// Balance is on the stack while it resolves, so the hand it counts is the
/// hand it left: cast out of two equal hands, it is your opponent who has to
/// discard.
#[test]
fn the_balance_itself_is_no_longer_in_the_hand_it_counts() {
    let (mut game, balance) = staged(&[cards::PLAINS], &[cards::ISLAND], (3, 3));
    assert_eq!(
        game.players[PlayerId::One.index()].hand.len(),
        4,
        "three cards and the Balance",
    );

    cast_and_settle(&mut game, balance);

    assert_eq!(
        (
            game.players[PlayerId::One.index()].hand.len(),
            game.players[PlayerId::Two.index()].hand.len()
        ),
        (3, 3),
        "it counted the three it left behind, so they came down to three",
    );
    assert_eq!(
        game.players[PlayerId::Two.index()].graveyard.len(),
        0,
        "which cost them nothing here: three was already three",
    );
}

/// "Cards in hand are counted after lands have been sacrificed, and
/// creatures on the battlefield are counted after cards have been discarded.
/// Thus, a land creature sacrificed to the first part of the spell would not
/// be counted when determining how many creatures are on the battlefield for
/// the last part."
///
/// The control comes first. Against a Forest and a bear, the lands step takes
/// their Forest and the creature step finds one body each, so both bears
/// live.
#[test]
fn a_land_creature_taken_by_the_first_step_is_gone_before_the_last() {
    let (mut game, balance) = staged(
        &[cards::GRIZZLY_BEARS],
        &[cards::FOREST, cards::GRIZZLY_BEARS],
        (0, 0),
    );

    cast_and_settle(&mut game, balance);

    assert_eq!(
        lands(&game, PlayerId::Two),
        0,
        "you had none, so they keep none"
    );
    assert_eq!(
        (
            creatures(&game, PlayerId::One),
            creatures(&game, PlayerId::Two)
        ),
        (1, 1),
        "one body each, so the creature step takes nothing",
    );

    // Now the same board with their bear replaced by a Dryad Arbor, which is
    // the land as well as the creature. It is sacrificed to the lands step,
    // so by the time creatures are counted they have none -- and the fewest
    // being zero takes your own bear with it.
    let (mut game, balance) = staged(&[cards::GRIZZLY_BEARS], &[cards::DRYAD_ARBOR], (0, 0));
    assert_eq!(
        (lands(&game, PlayerId::Two), creatures(&game, PlayerId::Two)),
        (1, 1),
        "the Arbor is both at once before anything resolves",
    );

    cast_and_settle(&mut game, balance);

    assert_eq!(
        lands(&game, PlayerId::Two),
        0,
        "the Arbor went to the lands step"
    );
    assert_eq!(
        creatures(&game, PlayerId::One),
        0,
        "and having already died it was not a creature to count, so the \
         fewest was none and your own bear went too",
    );
    assert!(
        game.battlefield.is_empty(),
        "nothing is left on either side",
    );
}
