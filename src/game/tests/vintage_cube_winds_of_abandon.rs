//! Winds of Abandon: two mana for one creature, six for the board, and a
//! basic land back for each one it takes.

use super::*;

/// Passes priority until the stack is empty.
fn resolve(game: &mut Game) {
    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
}

/// Cast normally it takes one creature, and its controller gets one basic
/// land back, tapped.
#[test]
fn winds_of_abandon_exiles_one_and_pays_a_land_for_it() {
    let mut game = ready_game();
    game.battlefield.clear();
    let bears = creature(93_000, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    game.players[1].library.clear();
    game.players[1]
        .library
        .push(card(93_001, cards::FOREST, PlayerId::Two));
    let winds = card(93_002, cards::WINDS_OF_ABANDON, PlayerId::One);
    let winds_id = winds.id;
    game.players[0].hand.push(winds);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == winds_id))
        .expect("two mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    resolve(&mut game);
    drain_pending(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == bears_id),
        "the creature is gone",
    );
    assert!(
        game.players[1]
            .exile
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS),
        "exiled rather than destroyed, so nothing rebuilds from it",
    );
    let forest = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::FOREST)
        .expect("its controller found a basic land");
    assert_eq!(forest.controller, PlayerId::Two);
    assert!(forest.tapped, "which arrives tapped");
}

/// Overloaded it takes every creature you don't control, leaves your own
/// alone, and pays one land per creature it took.
#[test]
fn overloaded_winds_takes_their_board_and_leaves_yours() {
    let mut game = ready_game();
    game.battlefield.clear();
    let mine = creature(93_010, cards::SAVANNAH_LIONS, PlayerId::One);
    let mine_id = mine.card.id;
    game.battlefield.push(mine);
    for id in 93_011..93_014 {
        game.battlefield
            .push(creature(id, cards::GRIZZLY_BEARS, PlayerId::Two));
    }
    game.players[1].library.clear();
    for id in 93_020..93_025 {
        game.players[1]
            .library
            .push(card(id, cards::FOREST, PlayerId::Two));
    }
    let winds = card(93_030, cards::WINDS_OF_ABANDON, PlayerId::One);
    let winds_id = winds.id;
    game.players[0].hand.push(winds);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 2);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 4);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == winds_id && choices.costs().alternative().is_some()
            }
            _ => false,
        })
        .expect("six mana overloads it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    resolve(&mut game);

    // "For each creature exiled this way" is three, not five: the count
    // comes from what was taken rather than from the library.
    let search = game
        .pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("their search asks which lands to take");
    assert_eq!(search.player, PlayerId::Two);
    assert_eq!(search.maximum, 3, "one land per creature exiled");
    game.apply(
        PlayerId::Two,
        Action::ChooseDecision {
            decision: search.id,
            options: search
                .options
                .iter()
                .take(3)
                .map(|option| option.id)
                .collect(),
        },
    )
    .expect("taking three is legal");
    drain_pending(&mut game);

    assert_eq!(
        game.players[1]
            .exile
            .iter()
            .filter(|card| card.definition == cards::GRIZZLY_BEARS)
            .count(),
        3,
        "every creature they controlled is exiled",
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == mine_id),
        "and your own is untouched",
    );
    assert_eq!(
        game.battlefield
            .iter()
            .filter(|permanent| permanent.card.definition == cards::FOREST)
            .count(),
        3,
        "three lands came back, tapped",
    );
}

/// "Because a spell with overload doesn't target when its overload cost is
/// paid, it may affect permanents with hexproof or with protection from the
/// appropriate color." A Sylvan Caryatid is not a creature the two-mana half
/// can name at all, and the six-mana half takes it without asking.
#[test]
fn overload_reaches_a_hexproof_creature_the_targeted_half_cannot() {
    let staged = |mana: (u16, u16)| {
        let mut game = ready_game();
        game.battlefield.clear();
        let caryatid = creature(93_040, cards::SYLVAN_CARYATID, PlayerId::Two);
        let caryatid_id = caryatid.card.id;
        game.battlefield.push(caryatid);
        game.players[1].library.clear();
        game.players[1]
            .library
            .push(card(93_041, cards::FOREST, PlayerId::Two));
        let winds = card(93_042, cards::WINDS_OF_ABANDON, PlayerId::One);
        let winds_id = winds.id;
        game.players[0].hand.push(winds);
        game.add_unrestricted_mana(PlayerId::One, ManaColor::White, mana.0);
        game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, mana.1);
        (game, winds_id, caryatid_id)
    };

    let (game, winds_id, caryatid_id) = staged((1, 1));
    assert!(
        !game.legal_actions(PlayerId::One).iter().any(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if *card == winds_id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(caryatid_id)))
        }),
        "two mana buys a target, and hexproof is not one",
    );

    let (mut game, winds_id, caryatid_id) = staged((2, 4));
    let overload = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == winds_id && choices.costs().alternative().is_some()
            }
            _ => false,
        })
        .expect("six mana overloads it");
    game.apply(PlayerId::One, overload).expect("it is cast");
    resolve(&mut game);
    drain_pending(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == caryatid_id),
        "and the overloaded half names nothing, so hexproof answers nothing",
    );
    assert!(
        game.players[1]
            .exile
            .iter()
            .any(|card| card.definition == cards::SYLVAN_CARYATID),
        "it is exiled like any other creature they control",
    );
}
