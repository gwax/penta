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

/// A library of `definitions` and nothing else, listed top of library first.
fn stacked(definitions: &[CardDefinitionId]) -> Game {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].library.clear();
    for (offset, definition) in definitions.iter().enumerate().rev() {
        let id = 81_000 + u32::try_from(offset).expect("a short library");
        game.players[0]
            .library
            .push(card(id, *definition, PlayerId::One));
    }
    drain_pending(&mut game);
    game
}

/// The library read from the top down, by the offsets [`stacked`] gave out.
fn from_the_top(game: &Game) -> Vec<u32> {
    game.players[0]
        .library
        .iter()
        .rev()
        .map(|card| card.id.0 - 81_000)
        .collect()
}

/// The Oracle's trigger with the Oracle in play and a library it cannot
/// empty: X cards seen, one answer given, and the stack settled after.
fn look_answering(game: &mut Game, chosen: Option<&str>) {
    game.put_onto_battlefield(PlayerId::One, cards::THASSAS_ORACLE)
        .expect("cataloged");
    let mut asked = None;
    for _ in 0..8 {
        asked = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone());
        if asked.is_some() || game.apply(game.priority, Action::PassPriority).is_err() {
            break;
        }
    }
    let decision = asked.expect("the trigger asks what stays on top");
    let options = chosen
        .map(|name| {
            vec![
                decision
                    .options
                    .iter()
                    .find(|option| option.label == name)
                    .unwrap_or_else(|| panic!("{name} was among the cards looked at"))
                    .id,
            ]
        })
        .unwrap_or_default();
    game.apply(
        decision.player,
        Action::ChooseDecision {
            decision: decision.id,
            options,
        },
    )
    .expect("the decision accepts what it offered");
    settle(game);
    drain_pending(game);
}

/// "Put up to one of them on top of your library and the rest on the bottom."
/// Devotion two looks at two cards; the one kept is the new top card, and the
/// one passed over goes under everything the Oracle never saw.
#[test]
fn the_card_kept_is_the_new_top_and_the_other_goes_under() {
    let mut game = stacked(&[
        cards::LORD_OF_ATLANTIS,
        cards::MOX_SAPPHIRE,
        cards::BLACK_LOTUS,
        cards::GRIZZLY_BEARS,
    ]);

    look_answering(&mut game, Some("Mox Sapphire"));

    assert_eq!(game.result, None, "four cards outrun two devotion");
    assert_eq!(
        from_the_top(&game),
        vec![1, 2, 3, 0],
        "the Mox stayed on top and the Lord went to the bottom",
    );
}

/// "Up to one" allows none of them, and then both looked-at cards are buried
/// and the third card down is what the next draw finds.
#[test]
fn keeping_none_of_them_buries_both() {
    let mut game = stacked(&[
        cards::LORD_OF_ATLANTIS,
        cards::MOX_SAPPHIRE,
        cards::BLACK_LOTUS,
        cards::GRIZZLY_BEARS,
    ]);

    look_answering(&mut game, None);

    let order = from_the_top(&game);
    assert_eq!(
        order[..2],
        [2, 3],
        "the two cards never looked at rose to the top",
    );
    let mut buried = order[2..].to_vec();
    buried.sort_unstable();
    assert_eq!(
        buried,
        vec![0, 1],
        "both looked-at cards went to the bottom"
    );
}

/// "If your devotion to blue is zero ... you don't look at or move any cards
/// in your library." An Oracle that has already left asks nothing, and the
/// library it triggered over is exactly as it was.
#[test]
fn zero_devotion_looks_at_nothing_at_all() {
    let mut game = stacked(&[
        cards::LORD_OF_ATLANTIS,
        cards::MOX_SAPPHIRE,
        cards::BLACK_LOTUS,
        cards::GRIZZLY_BEARS,
    ]);
    let oracle = game
        .put_onto_battlefield(PlayerId::One, cards::THASSAS_ORACLE)
        .expect("cataloged");
    game.battlefield
        .retain(|permanent| permanent.card.id != oracle);

    settle(&mut game);
    drain_pending(&mut game);

    assert_eq!(game.result, None, "four cards outrun no devotion");
    assert_eq!(
        from_the_top(&game),
        vec![0, 1, 2, 3],
        "nothing was looked at, so nothing moved",
    );
}
