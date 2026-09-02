//! Torsten, Founder of Benalia: seven mana that refills a hand on the way in
//! and leaves seven bodies on the way out.

use super::search_and_reveal::stack_library;
use super::*;

/// Torsten on the battlefield with `library` stacked top-first, and his
/// enters trigger still to be answered.
fn staged(library: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    stack_library(
        &mut game,
        &library
            .iter()
            .enumerate()
            .map(|(index, definition)| {
                (
                    67_000 + u32::try_from(index).expect("a handful of cards"),
                    *definition,
                )
            })
            .collect::<Vec<_>>(),
    );
    let torsten = game
        .put_onto_battlefield(PlayerId::One, cards::TORSTEN_FOUNDER_OF_BENALIA)
        .expect("cataloged");
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [7, 7];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    advance_to_decision(&mut game);
    (game, torsten)
}

/// Passes priority until the enters trigger asks its question.
fn advance_to_decision(game: &mut Game) {
    for _ in 0..12 {
        if !game.pending_decisions.is_empty() {
            return;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
}

/// What the enters trigger is offering to take.
fn offered(game: &Game) -> Vec<CardDefinitionId> {
    game.pending_decisions
        .first()
        .into_iter()
        .flat_map(|pending| pending.observation.options.iter())
        .filter_map(|option| {
            option
                .card
                .and_then(|(_, characteristics)| characteristics.card_definition())
        })
        .collect()
}

/// Answers the waiting decision with the options naming `wanted`.
fn take(game: &mut Game, wanted: &[CardDefinitionId]) {
    let decision = game
        .pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("the dig is asking");
    let mut left = wanted.to_vec();
    let options = decision
        .options
        .iter()
        .filter(|option| {
            option
                .card
                .and_then(|(_, characteristics)| characteristics.card_definition())
                .is_some_and(|definition| {
                    left.iter()
                        .position(|wanted| *wanted == definition)
                        .is_some_and(|index| {
                            left.remove(index);
                            true
                        })
                })
        })
        .map(|option| option.id)
        .collect::<Vec<_>>();
    game.apply(
        decision.player,
        Action::ChooseDecision {
            decision: decision.id,
            options,
        },
    )
    .expect("the choice is legal");
    drain_pending(game);
    game.check_state_based_actions();
}

fn in_hand(game: &Game, definition: CardDefinitionId) -> usize {
    game.players[0]
        .hand
        .iter()
        .filter(|card| card.definition == definition)
        .count()
}

/// Creatures and lands, and nothing else: the Bolt among the seven is not
/// offered.
#[test]
fn it_offers_creatures_and_lands_only() {
    let (game, _torsten) = staged(&[
        cards::GRIZZLY_BEARS,
        cards::LIGHTNING_BOLT,
        cards::FOREST,
        cards::ANCESTRAL_RECALL,
        cards::SERRA_ANGEL,
    ]);

    let mut found = offered(&game);
    found.sort_unstable();
    let mut wanted = vec![cards::GRIZZLY_BEARS, cards::FOREST, cards::SERRA_ANGEL];
    wanted.sort_unstable();
    assert_eq!(found, wanted);
}

/// "Any number" means the count is yours: take two of the three and the rest
/// goes to the bottom.
#[test]
fn any_number_of_them_is_a_real_choice() {
    let (mut game, _torsten) = staged(&[
        cards::GRIZZLY_BEARS,
        cards::LIGHTNING_BOLT,
        cards::FOREST,
        cards::SERRA_ANGEL,
    ]);

    take(&mut game, &[cards::GRIZZLY_BEARS, cards::SERRA_ANGEL]);

    assert_eq!(in_hand(&game, cards::GRIZZLY_BEARS), 1);
    assert_eq!(in_hand(&game, cards::SERRA_ANGEL), 1);
    assert_eq!(in_hand(&game, cards::FOREST), 0, "the Forest was declined");
    assert_eq!(game.players[0].hand.len(), 2, "and nothing else came");
    assert_eq!(
        game.players[0].library.len(),
        2,
        "the Bolt and the Forest went back",
    );
}

/// Taking nothing is allowed, and then all seven go to the bottom.
#[test]
fn none_of_them_is_also_a_number() {
    let (mut game, _torsten) = staged(&[cards::GRIZZLY_BEARS, cards::FOREST]);

    take(&mut game, &[]);

    assert!(game.players[0].hand.is_empty(), "nothing was taken");
    assert_eq!(game.players[0].library.len(), 2, "both went to the bottom");
}

/// It digs seven deep and no further.
#[test]
fn it_looks_seven_deep() {
    let deck = [cards::GRIZZLY_BEARS; 9];
    let (game, _torsten) = staged(&deck);

    assert_eq!(offered(&game).len(), 7, "seven cards, not nine");
}

/// Dying leaves seven 1/1 Soldiers behind.
#[test]
fn dying_leaves_seven_soldiers() {
    let (mut game, torsten) = staged(&[cards::FOREST]);
    take(&mut game, &[]);

    game.move_permanents_to_graveyard(&[torsten]);
    drain_pending(&mut game);
    game.check_state_based_actions();

    let soldiers = game
        .battlefield
        .iter()
        .filter(|permanent| {
            is_token_with(
                permanent,
                tokens::creature(&["Soldier"], &[ManaColor::White], 1, 1),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(soldiers.len(), 7, "seven of them");
    assert!(
        soldiers
            .iter()
            .all(|soldier| game.power(soldier) == Some(1)),
        "each a 1/1",
    );
}

/// "Reveal the top seven": all seven are shown to the table, not just the
/// ones taken. That is what the other player learns from the trigger, and it
/// costs nothing to leave a card behind after they have seen it.
#[test]
fn all_seven_are_revealed_whatever_is_taken() {
    let library = [
        cards::GRIZZLY_BEARS,
        cards::LIGHTNING_BOLT,
        cards::MOUNTAIN,
        cards::SERRA_ANGEL,
        cards::COUNTERSPELL,
        cards::FOREST,
        cards::PONDER,
    ];
    let (mut game, _torsten) = staged(&library);

    let revealed = |game: &Game| {
        let mut seen = game
            .events
            .iter()
            .filter_map(|event| match event {
                GameEvent::CardRevealed {
                    player, definition, ..
                } if *player == PlayerId::One => Some(*definition),
                _ => None,
            })
            .collect::<Vec<_>>();
        seen.sort_unstable();
        seen
    };
    let mut expected = library.to_vec();
    expected.sort_unstable();
    assert_eq!(
        revealed(&game),
        expected,
        "every one of the seven was shown before anything was chosen",
    );

    // Taking one of them adds nothing further: the reveal already happened.
    let before = revealed(&game).len();
    take(&mut game, &[cards::SERRA_ANGEL]);
    assert_eq!(
        revealed(&game).len(),
        before,
        "and the card taken is not revealed a second time",
    );
    assert_eq!(in_hand(&game, cards::SERRA_ANGEL), 1);
}

/// A library shorter than seven is shown as far as it goes: three cards is
/// three on offer, and taking them empties it.
#[test]
fn a_short_library_shows_what_is_there() {
    let (mut game, _torsten) = staged(&[cards::FOREST, cards::GRIZZLY_BEARS, cards::PLAINS]);

    assert_eq!(
        offered(&game).len(),
        3,
        "three cards is what there was to reveal",
    );
    take(
        &mut game,
        &[cards::FOREST, cards::GRIZZLY_BEARS, cards::PLAINS],
    );

    assert_eq!(in_hand(&game, cards::FOREST), 1);
    assert_eq!(in_hand(&game, cards::GRIZZLY_BEARS), 1);
    assert_eq!(in_hand(&game, cards::PLAINS), 1);
    assert!(
        game.players[0].library.is_empty(),
        "and nothing was left to put underneath",
    );
}

/// With nothing to reveal the trigger asks nothing at all, and he is on the
/// battlefield all the same.
#[test]
fn an_empty_library_asks_nothing() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    game.put_onto_battlefield(PlayerId::One, cards::TORSTEN_FOUNDER_OF_BENALIA)
        .expect("cataloged");
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [7, 7];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    drain_pending(&mut game);

    assert!(
        game.pending_decisions.is_empty(),
        "an empty library is nothing to choose from",
    );
    assert!(game.players[0].hand.is_empty(), "and nothing came of it");
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::TORSTEN_FOUNDER_OF_BENALIA),
        "while he is standing there regardless",
    );
}

/// Every Soldier this player controls, which is what the death trigger
/// leaves behind.
fn soldiers(game: &Game) -> usize {
    game.battlefield
        .iter()
        .filter(|permanent| {
            is_token_with(
                permanent,
                tokens::creature(&["Soldier"], &[ManaColor::White], 1, 1),
            )
        })
        .count()
}

/// "When Torsten dies" is the graveyard and nothing else: bouncing him to
/// hand leaves no Soldiers behind, which is how a blue deck answers him
/// without paying for it.
#[test]
fn bouncing_him_leaves_no_soldiers() {
    let (mut game, torsten) = staged(&[cards::FOREST]);
    take(&mut game, &[]);

    game.return_permanent_to_hand(torsten);
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert_eq!(soldiers(&game), 0, "he did not die, so nobody arrived");
    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::TORSTEN_FOUNDER_OF_BENALIA),
        "he is in hand instead",
    );
}

/// Exile is the other way of taking him off the board without the payment.
#[test]
fn exiling_him_leaves_no_soldiers() {
    let (mut game, torsten) = staged(&[cards::FOREST]);
    take(&mut game, &[]);

    game.exile_permanent(torsten);
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert_eq!(soldiers(&game), 0, "exile is not a graveyard");
    assert!(
        game.players[0]
            .exile
            .iter()
            .any(|card| card.definition == cards::TORSTEN_FOUNDER_OF_BENALIA),
        "and that is where he is",
    );
}

/// "When Torsten dies" is a battlefield event, and the third way round it is
/// the cheapest: a Torsten discarded to a Wheel of Fortune reaches the same
/// graveyard the death trigger watches and leaves nobody behind, because he
/// was never on the battlefield to die from.
#[test]
fn a_torsten_discarded_from_hand_leaves_no_soldiers() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    let torsten = card(84_800, cards::TORSTEN_FOUNDER_OF_BENALIA, PlayerId::One);
    game.players[0].hand.push(torsten);
    let wheel = card(84_801, cards::WHEEL_OF_FORTUNE, PlayerId::One);
    let wheel_id = wheel.id;
    game.players[0].hand.push(wheel);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 3);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == wheel_id))
        .expect("three mana casts the Wheel");
    game.apply(PlayerId::One, cast).expect("it is cast");
    pass_priority_pair(&mut game);
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::TORSTEN_FOUNDER_OF_BENALIA),
        "he was discarded into the graveyard",
    );
    assert_eq!(
        soldiers(&game),
        0,
        "and nobody arrived: a card in hand cannot die",
    );
}
