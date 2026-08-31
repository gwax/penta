//! Doomsday: five cards kept and everything else burned, for half of what
//! you have left.

use super::*;

/// Player One holding a Doomsday with `library` cards in the library,
/// `graveyard` in the graveyard, and `life` to spend.
fn staged(library: usize, graveyard: usize, life: i16) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    game.players[0].graveyard.clear();
    for index in 0..library {
        game.players[0].library.push(card(
            132_000 + u32::try_from(index).expect("a short library"),
            cards::GRIZZLY_BEARS,
            PlayerId::One,
        ));
    }
    for index in 0..graveyard {
        game.players[0].graveyard.push(card(
            132_100 + u32::try_from(index).expect("a short graveyard"),
            cards::LIGHTNING_BOLT,
            PlayerId::One,
        ));
    }
    let doomsday = card(132_200, cards::DOOMSDAY, PlayerId::One);
    let doomsday_id = doomsday.id;
    game.players[0].hand.push(doomsday);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 3);
    game.players[0].life = life;
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, doomsday_id)
}

/// Casts it and answers the search by keeping everything it offers.
fn cast_keeping_everything(game: &mut Game, doomsday: GameObjectId) -> usize {
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == doomsday))
        .expect("three black mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    pass_until_decision(game);
    let search = game
        .pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("the search asks which cards to keep");
    let offered = search.options.len();
    let chosen = search
        .options
        .iter()
        .map(|option| option.id)
        .collect::<Vec<_>>();
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: search.id,
            options: chosen,
        },
    )
    .expect("taking what there is is legal");
    drain_pending(game);
    offered
}

/// "If your graveyard and library combined contain fewer than five cards,
/// all of those cards will wind up in your library."
#[test]
fn a_pile_of_three_keeps_all_three() {
    let (mut game, doomsday) = staged(2, 1, 20);

    let offered = cast_keeping_everything(&mut game, doomsday);
    assert_eq!(offered, 3, "three cards between the two zones");

    assert_eq!(game.players[0].library.len(), 3, "and all three are kept");
    assert!(
        game.players[0].exile.is_empty(),
        "with nothing left over to exile",
    );
    assert_eq!(
        game.players[0]
            .graveyard
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::DOOMSDAY],
        "the graveyard holds the Doomsday and nothing it searched",
    );
    assert_eq!(game.players[0].life, 10, "the life is halved regardless");
}

/// "You lose half your life, rounded up": seven life costs four, not three.
#[test]
fn the_life_loss_rounds_up() {
    let (mut game, doomsday) = staged(2, 0, 7);

    cast_keeping_everything(&mut game, doomsday);

    assert_eq!(game.players[0].life, 3, "seven less four is three");
}

/// One life is one life: half of it rounded up is all of it.
#[test]
fn at_one_life_it_kills_you() {
    let (mut game, doomsday) = staged(2, 0, 1);

    cast_keeping_everything(&mut game, doomsday);
    game.check_state_based_actions();

    assert_eq!(game.players[0].life, 0);
    assert!(
        game.result.is_some(),
        "the search was the last thing it did"
    );
}

/// The line the card is played for: eight cards between the two zones, five
/// kept and the other three burned.
#[test]
fn five_are_kept_and_everything_else_is_exiled() {
    let (mut game, doomsday) = staged(6, 2, 20);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == doomsday))
        .expect("three black mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    pass_until_decision(&mut game);
    let search = game
        .pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("the search asks which five to keep");
    assert_eq!(search.options.len(), 8, "both zones are searched");
    assert_eq!(search.minimum, 5, "and five is not a maximum but a number");
    assert_eq!(search.maximum, 5);

    let five = search
        .options
        .iter()
        .take(5)
        .map(|option| option.id)
        .collect::<Vec<_>>();
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: search.id,
            options: five,
        },
    )
    .expect("five of the eight");
    drain_pending(&mut game);

    assert_eq!(
        game.players[0].library.len(),
        5,
        "five on top of the library"
    );
    assert_eq!(
        game.players[0].exile.len(),
        3,
        "and the three it did not want are exiled",
    );
    assert_eq!(
        game.players[0]
            .graveyard
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::DOOMSDAY],
        "the graveyard was searched too, and holds only the spell itself",
    );
    assert_eq!(game.players[0].life, 10);
}

/// "You must choose five cards from among them. You can't choose to find
/// fewer than that."
#[test]
fn four_of_the_eight_is_not_an_answer() {
    let (mut game, doomsday) = staged(6, 2, 20);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == doomsday))
        .expect("three black mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    pass_until_decision(&mut game);
    let search = game
        .pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("the search asks which five to keep");

    let four = search
        .options
        .iter()
        .take(4)
        .map(|option| option.id)
        .collect::<Vec<_>>();
    assert!(
        game.apply(
            PlayerId::One,
            Action::ChooseDecision {
                decision: search.id,
                options: four,
            },
        )
        .is_err(),
        "keeping four when five are there is not on offer",
    );
}
