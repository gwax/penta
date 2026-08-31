//! Thassa's Oracle: devotion counted, and a game ended by an empty library.

use super::*;

fn settle(game: &mut Game) {
    for _ in 0..16 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .first()
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

/// An empty board, a library of `library` cards, and `others` already down.
fn staged(library: usize, others: &[CardDefinitionId]) -> Game {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].library.clear();
    for offset in 0..library {
        let id = 81_000 + u32::try_from(offset).expect("a short library");
        game.players[0]
            .library
            .push(card(id, cards::GRIZZLY_BEARS, PlayerId::One));
    }
    for definition in others {
        game.put_onto_battlefield(PlayerId::One, *definition)
            .expect("cataloged");
    }
    drain_pending(&mut game);
    game
}

fn play_oracle(game: &mut Game) {
    game.put_onto_battlefield(PlayerId::One, cards::THASSAS_ORACLE)
        .expect("cataloged");
    settle(game);
    drain_pending(game);
}

/// An empty library and the Oracle's own two blue pips end the game.
#[test]
fn an_empty_library_wins_on_the_oracles_own_devotion() {
    let mut game = staged(0, &[]);

    play_oracle(&mut game);

    assert_eq!(
        game.result,
        Some(GameResult::Winner {
            winner: PlayerId::One,
            reason: WinReason::WonByAnEffect,
        }),
        "two devotion against no library",
    );
}

/// A library deeper than your devotion does not.
#[test]
fn a_deep_library_does_not_win() {
    let mut game = staged(10, &[]);

    play_oracle(&mut game);

    assert_eq!(game.result, None, "ten cards against two devotion");
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::THASSAS_ORACLE),
        "the Merfolk is just a Merfolk",
    );
}

/// Devotion counts every blue pip you control, not only the Oracle's own.
/// A Lord of Atlantis adds two more, which reaches a library of four that
/// the Oracle alone could not.
#[test]
fn devotion_counts_every_blue_permanent_you_control() {
    let mut alone = staged(4, &[]);
    play_oracle(&mut alone);
    assert_eq!(alone.result, None, "two devotion against four cards");

    let mut with_a_lord = staged(4, &[cards::LORD_OF_ATLANTIS]);
    play_oracle(&mut with_a_lord);

    assert_eq!(
        with_a_lord.result,
        Some(GameResult::Winner {
            winner: PlayerId::One,
            reason: WinReason::WonByAnEffect,
        }),
        "the Lord's two blue pips carry it to four",
    );
}

/// A permanent with no blue in its cost adds nothing, however many of them
/// there are.
#[test]
fn colorless_permanents_add_no_devotion() {
    let mut game = staged(4, &[cards::BLACK_LOTUS, cards::MOX_SAPPHIRE]);

    play_oracle(&mut game);

    assert_eq!(
        game.result, None,
        "neither artifact prints a blue mana symbol",
    );
}

/// Devotion is counted when the trigger resolves. If Oracle has left by then,
/// X is zero on an otherwise empty board: that still wins against an empty
/// library, but it does not reach a library containing one card.
#[test]
fn devotion_is_recounted_if_oracle_leaves_before_its_trigger_resolves() {
    for (library, expected_result) in [
        (
            0,
            Some(GameResult::Winner {
                winner: PlayerId::One,
                reason: WinReason::WonByAnEffect,
            }),
        ),
        (1, None),
    ] {
        let mut game = staged(library, &[]);
        let oracle = game
            .put_onto_battlefield(PlayerId::One, cards::THASSAS_ORACLE)
            .expect("cataloged");
        game.battlefield
            .retain(|permanent| permanent.card.id != oracle);

        settle(&mut game);
        drain_pending(&mut game);

        assert_eq!(
            game.result(),
            expected_result,
            "zero devotion against a library of {library}",
        );
    }
}

/// "Phyrexian mana symbols do count toward your devotion to their colour."
/// A Spined Thopter's {U/P} is a blue pip, whatever it was paid with.
#[test]
fn a_phyrexian_pip_counts_toward_devotion() {
    let mut game = staged(3, &[cards::SPINED_THOPTER]);

    play_oracle(&mut game);

    assert_eq!(
        game.result,
        Some(GameResult::Winner {
            winner: PlayerId::One,
            reason: WinReason::WonByAnEffect,
        }),
        "the Oracle's two and the Thopter's one reach three cards",
    );
}

/// "Mana symbols in the text boxes of permanents you control don't count."
/// A Talisman of Curiosity says {U} twice in its rules text and prints {2}
/// on top of the card, which is what devotion reads.
#[test]
fn a_blue_symbol_in_the_text_box_adds_nothing() {
    let mut game = staged(3, &[cards::TALISMAN_OF_CURIOSITY]);

    play_oracle(&mut game);

    assert_eq!(
        game.result, None,
        "two devotion against three cards, whatever the Talisman taps for",
    );
}

/// "Hybrid mana symbols ... do count toward your devotion to their
/// colour(s)." A Frostburn Weird is {U/R}{U/R}, and either half of a hybrid
/// pip is a blue pip for a devotion count that asks about blue.
#[test]
fn hybrid_pips_count_toward_the_colour_they_offer() {
    let mut game = staged(4, &[cards::FROSTBURN_WEIRD]);

    play_oracle(&mut game);

    assert_eq!(
        game.result,
        Some(GameResult::Winner {
            winner: PlayerId::One,
            reason: WinReason::WonByAnEffect,
        }),
        "the Oracle's two and the Weird's two reach four cards",
    );
}

/// "If you put an Aura on an opponent's permanent, you still control the
/// Aura, and mana symbols in its mana cost count towards your devotion."
/// Devotion asks who controls the permanent, and an Aura is a permanent of
/// its own however far across the table it is looking.
#[test]
fn an_aura_on_their_creature_is_still_your_devotion() {
    let mut game = staged(4, &[]);
    let bears = creature(81_500, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    let mut aura = creature(81_501, cards::INVISIBILITY, PlayerId::One);
    aura.attached_to = Some(bears_id);
    game.battlefield.push(aura);
    game.check_state_based_actions();
    drain_pending(&mut game);

    play_oracle(&mut game);

    assert_eq!(
        game.result,
        Some(GameResult::Winner {
            winner: PlayerId::One,
            reason: WinReason::WonByAnEffect,
        }),
        "the Invisibility is yours and its two blue pips are too",
    );
}
