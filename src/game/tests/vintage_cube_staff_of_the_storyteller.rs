//! Staff of the Storyteller: a flier on the way in, and a card for every
//! batch of creature tokens after it.

use super::*;
use crate::card::TokenCharacteristics;

/// The Staff on the battlefield since last turn, with a library to draw
/// from and one white mana up.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for index in 0..10 {
        game.players[0]
            .library
            .push(card(96_000 + index, cards::ISLAND, PlayerId::One));
    }
    let staff = game
        .put_onto_battlefield(PlayerId::One, cards::STAFF_OF_THE_STORYTELLER)
        .expect("cataloged");
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [2, 1];
    drain_pending(&mut game);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    (game, staff)
}

fn story(game: &Game, staff: GameObjectId) -> u16 {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == staff)
        .map_or(0, |permanent| permanent.counters(CounterKind::Story))
}

fn tokens(game: &Game) -> Vec<&Permanent> {
    game.battlefield
        .iter()
        .filter(|permanent| permanent.card.definition == ObjectKind::Token)
        .collect()
}

fn draws(game: &Game, staff: GameObjectId) -> Vec<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == staff),
        )
        .collect()
}

/// It arrives with a Spirit, and its own trigger counts that Spirit.
#[test]
fn it_enters_with_a_spirit_and_a_story_counter() {
    let (game, staff) = staged();

    let made = tokens(&game);
    assert_eq!(made.len(), 1, "one Spirit");
    assert_eq!(game.power(made[0]), Some(1));
    assert_eq!(game.toughness(made[0]), Some(1));
    assert!(game.has_flying(made[0]), "with flying");
    assert_eq!(
        story(&game, staff),
        1,
        "and the Staff counted its own token",
    );
}

/// A counter and the tap buy a card.
#[test]
fn a_story_counter_buys_a_card() {
    let (mut game, staff) = staged();
    let hand = game.players[0].hand.len();

    let draw = draws(&game, staff)
        .into_iter()
        .next()
        .expect("a white and the tap draw");
    game.apply(PlayerId::One, draw).expect("it activates");
    drain_pending(&mut game);

    assert_eq!(game.players[0].hand.len(), hand + 1, "a card was drawn");
    assert_eq!(story(&game, staff), 0, "the counter paid for it");
    assert!(
        draws(&game, staff).is_empty(),
        "and there is neither a counter nor an untapped Staff for another",
    );
}

/// "One or more": two Spirits from one Lingering Souls is one counter.
#[test]
fn one_instruction_is_one_counter_however_many_tokens() {
    let (mut game, staff) = staged();
    let souls = card(96_500, cards::LINGERING_SOULS, PlayerId::One);
    let souls_id = souls.id;
    game.players[0].hand.push(souls);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == souls_id))
        .expect("three mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);

    assert_eq!(tokens(&game).len(), 3, "the Staff's Spirit and two more");
    assert_eq!(
        story(&game, staff),
        2,
        "one counter for the pair, on top of the Staff's own",
    );
}

/// The clause names creature tokens: a Treasure is not one.
#[test]
fn a_noncreature_token_is_not_counted() {
    let (mut game, staff) = staged();

    game.create_token(PlayerId::One, crate::card::tokens::treasure());
    drain_pending(&mut game);

    assert_eq!(story(&game, staff), 1, "still only the Spirit's counter");
}

/// And it names yours: their tokens are their business.
#[test]
fn their_tokens_are_not_counted() {
    let (mut game, staff) = staged();

    game.create_token(
        PlayerId::Two,
        TokenCharacteristics::creature(&["Zombie"], &[ManaColor::Black], 2, 2),
    );
    drain_pending(&mut game);

    assert_eq!(story(&game, staff), 1, "still only the Spirit's counter");
}

/// The tap costs nothing but a tap: an artifact has no summoning sickness
/// (CR 302.6 is about creatures), so the Staff cashes the counter its own
/// Spirit gave it on the turn it lands.
#[test]
fn it_draws_the_turn_it_arrives() {
    let (mut game, staff) = staged();
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = game.turns_started[0];
    }
    let hand = game.players[0].hand.len();
    assert_eq!(story(&game, staff), 1, "the Spirit it came with paid it");

    let draw = draws(&game, staff)
        .into_iter()
        .next()
        .expect("nothing about the Staff waits a turn");
    game.apply(PlayerId::One, draw).expect("it activates");
    drain_pending(&mut game);

    assert_eq!(
        game.players[0].hand.len(),
        hand + 1,
        "a card, the same turn"
    );
}

/// A story counter is a counter like any other: proliferate finds it and
/// adds one, which is a second card the Staff never had to make a token
/// for.
#[test]
fn proliferate_adds_a_story_counter() {
    let (mut game, staff) = staged();
    assert_eq!(story(&game, staff), 1, "the Spirit it came with");
    let progress = game
        .build_zone(PlayerId::One, &[cards::STEADY_PROGRESS])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let progress_id = progress.id;
    game.players[0].hand.push(progress);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == progress_id))
        .expect("three mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    for _ in 0..12 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .filter(|option| option.card.is_some_and(|(id, _)| id == staff))
                .map(|option| option.id)
                .collect::<Vec<_>>();
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the Staff is one of the permanents it may add to");
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

    assert_eq!(
        story(&game, staff),
        2,
        "the proliferate put a second story counter on it",
    );
    assert_eq!(
        draws(&game, staff).len(),
        1,
        "and the tap is still the limit on spending them",
    );
}
