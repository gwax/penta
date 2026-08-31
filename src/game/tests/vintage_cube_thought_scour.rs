//! Thought Scour: two cards off a library and one into your hand.
//!
//! The order the card prints is the order it happens in, which is the whole
//! of what the card does differently from a cantrip: point it at yourself
//! and the card you draw is the one under the two you just lost.

use super::*;

/// Player One holding a Thought Scour with the mana for it, and `library`
/// stacked top-first under `player`.
fn staged(player: PlayerId, library: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game.players[player.index()].library.clear();
    game.players[player.index()].graveyard.clear();
    // A library's last element is its top, so a top-first list goes in
    // backwards.
    for (index, definition) in library.iter().rev().enumerate() {
        game.players[player.index()].library.push(card(
            95_000 + u32::try_from(index).expect("a short library"),
            *definition,
            player,
        ));
    }
    let scour = card(95_500, cards::THOUGHT_SCOUR, PlayerId::One);
    let scour_id = scour.id;
    game.players[PlayerId::One.index()].hand.push(scour);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, scour_id)
}

fn cast_at(game: &mut Game, scour: GameObjectId, player: PlayerId) {
    game.apply(
        PlayerId::One,
        cast_action(scour, vec![Target::Player(player)], Vec::new(), 0),
    )
    .expect("a player is what it targets");
    drain_pending(game);
}

fn graveyard(game: &Game, player: PlayerId) -> Vec<CardDefinitionId> {
    game.players[player.index()]
        .graveyard
        .iter()
        .map(|card| card.definition)
        .filter(|definition| *definition != cards::THOUGHT_SCOUR)
        .collect()
}

fn hand(game: &Game) -> Vec<CardDefinitionId> {
    game.players[PlayerId::One.index()]
        .hand
        .iter()
        .map(|card| card.definition)
        .collect()
}

/// "Follow the instructions in the order listed on the card: if you target
/// yourself, you'll put the top two cards of your library into your
/// graveyard and then draw a card." The third card down is the one that
/// comes, not the first.
#[test]
fn pointed_at_yourself_it_mills_first_and_draws_after() {
    let (mut game, scour) = staged(
        PlayerId::One,
        &[
            cards::LIGHTNING_BOLT,
            cards::GRIZZLY_BEARS,
            cards::SERRA_ANGEL,
            cards::FOREST,
        ],
    );

    cast_at(&mut game, scour, PlayerId::One);

    assert_eq!(
        graveyard(&game, PlayerId::One),
        vec![cards::LIGHTNING_BOLT, cards::GRIZZLY_BEARS],
        "the top two went first",
    );
    assert_eq!(
        hand(&game),
        vec![cards::SERRA_ANGEL],
        "so the draw is what was under them",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].library.len(),
        1,
        "three cards left the library in all",
    );
}

/// Pointed across the table it is the same two cards off their library, and
/// the draw is still yours.
#[test]
fn pointed_at_them_it_still_draws_for_you() {
    let (mut game, scour) = staged(
        PlayerId::Two,
        &[cards::COUNTERSPELL, cards::ISLAND, cards::SERRA_ANGEL],
    );
    let theirs = game.players[PlayerId::Two.index()].hand.len();

    cast_at(&mut game, scour, PlayerId::Two);

    assert_eq!(
        graveyard(&game, PlayerId::Two),
        vec![cards::COUNTERSPELL, cards::ISLAND],
        "their top two",
    );
    assert_eq!(
        game.players[PlayerId::Two.index()].hand.len(),
        theirs,
        "and nothing of it reaches their hand",
    );
    assert_eq!(hand(&game).len(), 1, "the card drawn is yours");
}

/// "Mills two cards" with one card there mills the one. Milling out is not
/// itself losing: the empty library is only fatal to the player who has to
/// draw from it.
#[test]
fn it_mills_what_there_is_and_no_more() {
    let (mut game, scour) = staged(PlayerId::Two, &[cards::COUNTERSPELL]);

    cast_at(&mut game, scour, PlayerId::Two);

    assert_eq!(graveyard(&game, PlayerId::Two), vec![cards::COUNTERSPELL]);
    assert!(game.players[PlayerId::Two.index()].library.is_empty());
    assert_eq!(
        game.result, None,
        "an empty library is not a loss until it is drawn from",
    );
    assert_eq!(hand(&game).len(), 1, "and your own draw was never in doubt");
}

/// A player with hexproof is no target for their opponent's spell, and is
/// still every bit a target for their own: the Scour may name its caster
/// while it may not name them.
#[test]
fn a_hexproof_player_is_no_target_for_their_opponent() {
    let (mut game, scour) = staged(PlayerId::Two, &[cards::FOREST, cards::ISLAND]);
    game.put_onto_battlefield(PlayerId::Two, cards::LEYLINE_OF_SANCTITY)
        .expect("cataloged");
    drain_pending(&mut game);

    let named = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell { card, choices, .. } if card == scour => Some(
                choices
                    .iter_targets()
                    .filter_map(|target| match target {
                        Target::Player(player) => Some(*player),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .flatten()
        .collect::<Vec<_>>();

    assert_eq!(
        named,
        vec![PlayerId::One],
        "the Leyline takes its controller off the list and leaves the caster on it",
    );
}

/// "If the target player is an illegal target when Thought Scour tries to
/// resolve, it won't resolve and none of its effects will happen. You won't
/// draw a card." The mill and the draw are one spell, and hexproof arriving
/// in between takes both.
#[test]
fn a_target_that_gains_hexproof_takes_the_draw_with_it() {
    let (mut game, scour) = staged(PlayerId::Two, &[cards::FOREST, cards::ISLAND]);
    let held = hand(&game).len();
    let library = game.players[PlayerId::One.index()].library.len();

    game.apply(
        PlayerId::One,
        cast_action(scour, vec![Target::Player(PlayerId::Two)], Vec::new(), 0),
    )
    .expect("they are a legal target when it is cast");

    // In response, with the Scour still on the stack.
    game.put_onto_battlefield(PlayerId::Two, cards::LEYLINE_OF_SANCTITY)
        .expect("cataloged");
    drain_pending(&mut game);

    assert!(
        graveyard(&game, PlayerId::Two).is_empty(),
        "nothing was milled: the spell had no legal target left",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].library.len(),
        library,
        "and the draw went with it",
    );
    assert_eq!(
        hand(&game).len(),
        held - 1,
        "the Scour itself is all that left the caster's hand",
    );
    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::THOUGHT_SCOUR),
        "and it is in the graveyard, countered on resolution",
    );
}
