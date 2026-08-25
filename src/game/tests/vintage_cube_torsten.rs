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
