//! Faithless Looting: one red for two cards, two cards, and a second use.
//!
//! Flashback as a mechanic is exercised across the suite. What is here is
//! this card: the loot itself, and what its own flashback cost buys.

use super::*;

/// Player One holding a Looting with `library` stacked and `hand` beside it.
fn staged(hand: &[CardDefinitionId], library: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    game.players[0].graveyard.clear();
    game.players[0].exile.clear();
    // The top of a library is its last element.
    for (index, definition) in library.iter().enumerate().rev() {
        game.players[0].library.push(card(
            99_500 + u32::try_from(index).expect("a short library"),
            *definition,
            PlayerId::One,
        ));
    }
    for (index, definition) in hand.iter().enumerate() {
        game.players[0].hand.push(card(
            99_600 + u32::try_from(index).expect("a small hand"),
            *definition,
            PlayerId::One,
        ));
    }
    let looting = card(99_700, cards::FAITHLESS_LOOTING, PlayerId::One);
    let looting_id = looting.id;
    game.players[0].hand.push(looting);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, looting_id)
}

/// Resolves what is on the stack, discarding the cards whose definitions are
/// in `bin` when the discard asks.
fn settle_discarding(game: &mut Game, bin: &[CardDefinitionId]) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let wanted: Vec<_> = decision
                .options
                .iter()
                .filter(|option| {
                    option.card.is_some_and(|(_, characteristics)| {
                        characteristics
                            .card_definition()
                            .is_some_and(|definition| bin.contains(&definition))
                    })
                })
                .map(|option| option.id)
                .take(decision.maximum)
                .collect();
            let options = if wanted.is_empty() {
                decision
                    .options
                    .iter()
                    .map(|option| option.id)
                    .take(decision.minimum)
                    .collect()
            } else {
                wanted
            };
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
    drain_pending(game);
}

/// Every way the Looting is castable right now, from wherever it is.
fn casts(game: &Game, looting: GameObjectId) -> Vec<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(|action| matches!(action, Action::CastSpell { card, .. } if *card == looting))
        .collect()
}

/// "Draw two cards, then discard two cards": the hand is the size it was,
/// and what goes is the caster's pick -- including the cards just drawn,
/// since by then they are cards in hand like any other.
#[test]
fn it_draws_two_and_bins_two_of_your_choosing() {
    let (mut game, looting) = staged(
        &[cards::SERRA_ANGEL, cards::GRIZZLY_BEARS],
        &[cards::MOUNTAIN, cards::ISLAND, cards::FOREST],
    );
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);

    let cast = casts(&game, looting)
        .into_iter()
        .next()
        .expect("one red casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle_discarding(&mut game, &[cards::MOUNTAIN, cards::ISLAND]);

    assert_eq!(
        game.players[0].library.len(),
        1,
        "two cards came off the library",
    );
    assert_eq!(
        game.players[0].hand.len(),
        2,
        "and the hand is the size it started, the Looting having left it",
    );
    let mut kept: Vec<_> = game.players[0]
        .hand
        .iter()
        .map(|card| card.definition)
        .collect();
    kept.sort_unstable();
    let mut wanted = vec![cards::SERRA_ANGEL, cards::GRIZZLY_BEARS];
    wanted.sort_unstable();
    assert_eq!(kept, wanted, "the two it started with are the two it kept");
    for definition in [cards::MOUNTAIN, cards::ISLAND, cards::FAITHLESS_LOOTING] {
        assert!(
            game.players[0]
                .graveyard
                .iter()
                .any(|card| card.definition == definition),
            "{definition:?} is in the graveyard",
        );
    }
}

/// Flashback {2}{R}: the same spell a second time out of the graveyard, and
/// the card is exiled rather than going back to it. Which is what makes the
/// one-mana sorcery worth a slot.
#[test]
fn flashback_casts_it_again_and_exiles_it() {
    let (mut game, _held) = staged(&[], &[cards::MOUNTAIN, cards::ISLAND, cards::FOREST]);
    // Put it in the graveyard rather than the hand: this is the second cast.
    let looting = game.players[0].hand.pop().expect("the Looting");
    let looting_id = looting.id;
    game.players[0].graveyard.push(looting);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);

    let cast = casts(&game, looting_id)
        .into_iter()
        .next()
        .expect("three mana flashes it back");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle_discarding(&mut game, &[cards::MOUNTAIN, cards::ISLAND]);

    assert_eq!(
        game.players[0].library.len(),
        1,
        "it drew its two out of the graveyard just the same",
    );
    assert!(
        game.players[0]
            .exile
            .iter()
            .any(|card| card.definition == cards::FAITHLESS_LOOTING),
        "and flashback exiled it afterwards",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .all(|card| card.definition != cards::FAITHLESS_LOOTING),
        "rather than leaving it to be cast a third time",
    );
}

/// "You must still follow any timing restrictions, including those based on
/// the card's type: you can cast a sorcery using flashback only when you
/// could normally cast a sorcery."
#[test]
fn the_flashback_cast_is_still_a_sorcery() {
    let (mut game, _held) = staged(&[], &[cards::MOUNTAIN, cards::ISLAND]);
    let looting = game.players[0].hand.pop().expect("the Looting");
    let looting_id = looting.id;
    game.players[0].graveyard.push(looting);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);

    assert!(
        !casts(&game, looting_id).is_empty(),
        "your own main phase, and the stack empty",
    );

    game.step = Step::DeclareBlockers;
    assert!(
        casts(&game, looting_id).is_empty(),
        "combat is no time for a sorcery, flashback or not",
    );

    game.step = Step::PrecombatMain;
    game.active_player = PlayerId::Two;
    game.turns_started = [5, 6];
    assert!(
        casts(&game, looting_id).is_empty(),
        "and neither is a main phase of theirs",
    );
}
