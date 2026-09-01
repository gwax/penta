//! Consult the Star Charts: a look as deep as your mana base, and twice the
//! keeping when it is kicked.

use super::*;

/// The spell in hand with `lands` lands out and `library` on top.
fn staged(lands: usize, library: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    // The back of the library is the top, so the first named card is drawn
    // into the look first.
    for definition in library.iter().rev() {
        let card = game
            .build_zone(PlayerId::One, &[*definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        game.players[0].library.push(card);
    }
    for index in 0..lands {
        game.battlefield.push(creature(
            99_000 + u32::try_from(index).expect("few lands"),
            cards::ISLAND,
            PlayerId::One,
        ));
    }
    let consult = game
        .build_zone(PlayerId::One, &[cards::CONSULT_THE_STAR_CHARTS])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let consult_id = consult.id;
    game.players[0].hand.push(consult);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 4);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, consult_id)
}

/// The cast, kicked or not.
fn cast(game: &mut Game, consult: GameObjectId, kicked: bool) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if *card == consult && choices.costs().alternative().is_some() == kicked)
        })
        .unwrap_or_else(|| panic!("it is castable (kicked: {kicked})"));
    game.apply(PlayerId::One, action).expect("it is cast");
    for _ in 0..12 {
        if game.observe(PlayerId::One).decision.is_some() {
            return;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
}

fn hand(game: &Game) -> Vec<CardDefinitionId> {
    game.players[0]
        .hand
        .iter()
        .map(|card| card.definition)
        .collect()
}

/// X is the number of lands: three lands look at three cards, and one of
/// them is kept.
#[test]
fn it_looks_as_deep_as_your_lands_and_keeps_one() {
    let (mut game, consult) = staged(
        3,
        &[
            cards::BLACK_LOTUS,
            cards::MOX_JET,
            cards::GRIZZLY_BEARS,
            cards::SERRA_ANGEL,
        ],
    );

    cast(&mut game, consult, false);
    let decision = game
        .observe(PlayerId::One)
        .decision
        .expect("the look asks what to keep");
    assert_eq!(decision.options.len(), 3, "three lands, three cards seen");
    assert_eq!(decision.minimum, 1);
    assert_eq!(decision.maximum, 1, "one of them, unkicked");

    let lotus = decision
        .options
        .iter()
        .find(|option| {
            option.card.is_some_and(|(_, characteristics)| {
                characteristics.card_definition() == Some(cards::BLACK_LOTUS)
            })
        })
        .expect("the Lotus was among them")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![lotus],
        },
    )
    .expect("keeping it is legal");

    assert_eq!(hand(&game), vec![cards::BLACK_LOTUS]);
    assert_eq!(
        game.players[0].library.len(),
        3,
        "the two it passed over went back under the one it never saw",
    );
}

/// Kicked, it keeps two.
#[test]
fn kicked_it_keeps_two() {
    let (mut game, consult) = staged(
        3,
        &[cards::BLACK_LOTUS, cards::MOX_JET, cards::GRIZZLY_BEARS],
    );

    cast(&mut game, consult, true);
    let decision = game
        .observe(PlayerId::One)
        .decision
        .expect("the look asks what to keep");
    assert_eq!(decision.minimum, 2);
    assert_eq!(decision.maximum, 2, "two of them, kicked");

    let chosen = decision
        .options
        .iter()
        .take(2)
        .map(|option| option.id)
        .collect();
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: chosen,
        },
    )
    .expect("keeping two is legal");

    assert_eq!(hand(&game).len(), 2);
}

/// With no lands there is nothing to look at, and the spell simply resolves.
#[test]
fn no_lands_looks_at_nothing() {
    let (mut game, consult) = staged(0, &[cards::BLACK_LOTUS]);

    cast(&mut game, consult, false);

    assert!(game.observe(PlayerId::One).decision.is_none());
    assert!(hand(&game).is_empty(), "nothing was kept");
    assert_eq!(game.players[0].library.len(), 1);
}

/// "The value of X is calculated only once, as Consult the Star Charts
/// resolves": a land that arrives while it is on the stack is one more card
/// seen.
#[test]
fn x_is_read_when_it_resolves_rather_than_when_it_is_cast() {
    let (mut game, consult) = staged(
        2,
        &[
            cards::BLACK_LOTUS,
            cards::MOX_JET,
            cards::GRIZZLY_BEARS,
            cards::SERRA_ANGEL,
        ],
    );

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if *card == consult && choices.costs().alternative().is_none())
        })
        .expect("two mana casts it unkicked");
    game.apply(PlayerId::One, action).expect("it is cast");

    // In response, a third land: X has not been read yet.
    game.put_onto_battlefield(PlayerId::One, cards::ISLAND)
        .expect("cataloged");
    for _ in 0..12 {
        if game.observe(PlayerId::One).decision.is_some() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }

    let decision = game
        .observe(PlayerId::One)
        .decision
        .expect("the look asks what to keep");
    assert_eq!(
        decision.options.len(),
        3,
        "three lands by the time it resolved, so three cards seen",
    );
}

/// "The number of lands *you control*": theirs are none of yours.
#[test]
fn their_lands_are_not_counted() {
    let (mut game, consult) = staged(
        1,
        &[
            cards::BLACK_LOTUS,
            cards::MOX_JET,
            cards::GRIZZLY_BEARS,
            cards::SERRA_ANGEL,
        ],
    );
    for index in 0..4 {
        game.battlefield
            .push(creature(99_500 + index, cards::MOUNTAIN, PlayerId::Two));
    }
    game.priority = PlayerId::One;

    cast(&mut game, consult, false);

    // One land is one card seen, and one card seen is no choice to make:
    // nobody is asked, and the top card is simply taken.
    assert!(
        game.observe(PlayerId::One).decision.is_none(),
        "their four Mountains would have made this a choice among five",
    );
    assert_eq!(
        hand(&game),
        vec![cards::BLACK_LOTUS],
        "your one Island is the whole of X",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].library.len(),
        3,
        "and the rest of the library was never looked at",
    );
}
