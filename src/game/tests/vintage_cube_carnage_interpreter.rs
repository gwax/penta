//! Carnage Interpreter: a hand spent for four Clues, and a body that is
//! bigger exactly while the hand stays spent.

use super::*;

/// The Interpreter about to enter, with `hand` cards in Player One's hand.
fn staged(hand: &[CardDefinitionId]) -> Game {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for (index, definition) in [cards::MOUNTAIN, cards::FOREST].into_iter().enumerate() {
        let id = 282_000 + u32::try_from(index).expect("two cards");
        game.players[0]
            .library
            .push(card(id, definition, PlayerId::One));
    }
    for (index, definition) in hand.iter().enumerate() {
        let id = 282_100 + u32::try_from(index).expect("a short hand");
        game.players[0]
            .hand
            .push(card(id, *definition, PlayerId::One));
    }
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game
}

fn settle(game: &mut Game) {
    for _ in 0..32 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .map(|option| option.id)
                .take(decision.minimum.max(1))
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

fn arrive(game: &mut Game) -> GameObjectId {
    let interpreter = game
        .put_onto_battlefield(PlayerId::One, cards::CARNAGE_INTERPRETER)
        .expect("cataloged");
    settle(game);
    interpreter
}

fn clues(game: &Game) -> usize {
    game.battlefield
        .iter()
        .filter(|permanent| permanent.card.definition == ObjectKind::Token)
        .count()
}

/// Arriving empties the hand and leaves four Clues behind.
#[test]
fn it_spends_the_hand_for_four_clues() {
    let mut game = staged(&[cards::LIGHTNING_BOLT, cards::SERRA_ANGEL, cards::MOUNTAIN]);

    arrive(&mut game);

    assert!(game.players[0].hand.is_empty(), "the whole hand went");
    assert_eq!(game.players[0].graveyard.len(), 3, "all three of it");
    assert_eq!(clues(&game), 4, "and four Clues arrived");
}

/// With the hand spent it is a 5/5 with menace.
#[test]
fn an_empty_hand_makes_it_a_five_five_with_menace() {
    let mut game = staged(&[cards::LIGHTNING_BOLT]);

    let interpreter = arrive(&mut game);

    let permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == interpreter)
        .expect("it is there");
    assert_eq!(game.power(permanent), Some(5));
    assert_eq!(game.toughness(permanent), Some(5));
    assert!(game.permanent_has_executable_keyword(permanent, KeywordAbility::Menace));
}

/// One card in hand is still few enough.
#[test]
fn one_card_in_hand_is_still_few_enough() {
    let mut game = staged(&[]);
    let interpreter = arrive(&mut game);
    game.players[0]
        .hand
        .push(card(282_200, cards::MOUNTAIN, PlayerId::One));

    let permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == interpreter)
        .expect("it is there");
    assert_eq!(game.power(permanent), Some(5), "one card is one or fewer");
}

/// A second card turns the bonus off again: the condition is read live.
#[test]
fn a_refilled_hand_turns_the_bonus_off() {
    let mut game = staged(&[]);
    let interpreter = arrive(&mut game);
    for index in 0..2 {
        game.players[0]
            .hand
            .push(card(282_300 + index, cards::MOUNTAIN, PlayerId::One));
    }

    let permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == interpreter)
        .expect("it is there");
    assert_eq!(game.power(permanent), Some(3), "back to its printed body");
    assert_eq!(game.toughness(permanent), Some(3));
    assert!(!game.permanent_has_executable_keyword(permanent, KeywordAbility::Menace));
}

/// The Clues do what Clues do.
#[test]
fn a_clue_draws_a_card() {
    let mut game = staged(&[]);
    arrive(&mut game);
    let clue = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == ObjectKind::Token)
        .map(|permanent| permanent.card.id)
        .expect("a Clue is there");
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);
    let before = game.players[0].library.len();

    let activation = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == clue))
        .expect("two mana and a sacrifice draws");
    game.apply(PlayerId::One, activation)
        .expect("it is activated");
    settle(&mut game);

    assert_eq!(game.players[0].hand.len(), 1, "a card was drawn");
    assert_eq!(game.players[0].library.len(), before - 1);
    assert_eq!(clues(&game), 3, "and the Clue was spent");
}
