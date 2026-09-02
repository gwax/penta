//! Expressive Iteration: three cards, three places, and one of them has to
//! be spent before the turn ends.

use super::*;

/// The Iteration in hand with two mana up and a known top three.
fn staged(top: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for index in 0..4 {
        game.players[0].library.push(card(
            94_000 + u32::try_from(index).expect("small"),
            cards::SWAMP,
            PlayerId::One,
        ));
    }
    // The library reads from the back, so the dig is pushed last.
    for (index, definition) in top.iter().rev().enumerate() {
        game.players[0].library.push(card(
            94_500 + u32::try_from(index).expect("small"),
            *definition,
            PlayerId::One,
        ));
    }
    let iteration = game
        .build_zone(PlayerId::One, &[cards::EXPRESSIVE_ITERATION])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let iteration_id = iteration.id;
    game.players[0].hand.push(iteration);
    game.turns_started = [2, 1];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    (game, iteration_id)
}

/// Casts it, answering each question with the card `pick` names, and
/// recording the prompts it was asked.
fn cast_iteration(
    game: &mut Game,
    iteration: GameObjectId,
    mut pick: impl FnMut(&str, &[CardDefinitionId]) -> usize,
) -> Vec<String> {
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == iteration))
        .expect("two mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    let mut asked = Vec::new();
    for _ in 0..16 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let offered = decision
                .options
                .iter()
                .filter_map(|option| option.card.map(|(_, card)| card))
                .filter_map(ObjectCharacteristics::card_definition)
                .collect::<Vec<_>>();
            let index = pick(&decision.prompt, &offered);
            asked.push(decision.prompt.clone());
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options: vec![decision.options[index].id],
                },
            )
            .expect("the choice is legal");
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
    asked
}

fn hand(game: &Game) -> Vec<CardDefinitionId> {
    game.players[0]
        .hand
        .iter()
        .map(|card| card.definition)
        .collect()
}

fn exile(game: &Game) -> Vec<CardDefinitionId> {
    game.players[0]
        .exile
        .iter()
        .map(|card| card.definition)
        .collect()
}

fn library_from_bottom(game: &Game) -> Vec<CardDefinitionId> {
    game.players[0]
        .library
        .iter()
        .map(|card| card.definition)
        .collect()
}

/// Two questions and a forced third: the hand card, then the one that goes
/// underneath, and what is left is exiled.
#[test]
fn each_of_the_three_goes_somewhere_different() {
    let (mut game, iteration) =
        staged(&[cards::LIGHTNING_BOLT, cards::ISLAND, cards::GRIZZLY_BEARS]);

    let asked = cast_iteration(&mut game, iteration, |prompt, offered| {
        if prompt.contains("hand") {
            offered
                .iter()
                .position(|definition| *definition == cards::LIGHTNING_BOLT)
                .expect("the Bolt is on offer")
        } else {
            offered
                .iter()
                .position(|definition| *definition == cards::ISLAND)
                .expect("the Island is on offer")
        }
    });

    assert_eq!(
        asked,
        [
            "Put a card into your hand",
            "Put a card on the bottom of your library",
        ],
        "the last card has nothing left to decide",
    );
    assert_eq!(hand(&game), vec![cards::LIGHTNING_BOLT]);
    assert_eq!(exile(&game), vec![cards::GRIZZLY_BEARS]);
    assert_eq!(
        library_from_bottom(&game)[0],
        cards::ISLAND,
        "and the one nobody wanted is underneath everything",
    );
}

/// The exiled card is playable for the rest of the turn, and it is the only
/// one of the three that is.
#[test]
fn the_exiled_card_may_be_played_this_turn() {
    let (mut game, iteration) = staged(&[cards::ISLAND, cards::SWAMP, cards::LIGHTNING_BOLT]);
    game.players[0].lands_played_this_turn = 0;

    cast_iteration(&mut game, iteration, |prompt, offered| {
        if prompt.contains("hand") {
            offered
                .iter()
                .position(|definition| *definition == cards::SWAMP)
                .expect("the Swamp is on offer")
        } else {
            offered
                .iter()
                .position(|definition| *definition == cards::LIGHTNING_BOLT)
                .expect("the Bolt is on offer")
        }
    });

    assert_eq!(exile(&game), vec![cards::ISLAND]);
    let exiled = game.players[0].exile[0].id;
    assert!(
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::PlayLand { card, .. } if *card == exiled)),
        "the exiled land can be played out of exile",
    );
}

/// The permission ends with the turn: what was not played is stuck there.
#[test]
fn the_permission_lapses_at_end_of_turn() {
    let (mut game, iteration) = staged(&[cards::ISLAND, cards::SWAMP, cards::LIGHTNING_BOLT]);

    cast_iteration(&mut game, iteration, |prompt, offered| {
        if prompt.contains("hand") {
            offered
                .iter()
                .position(|definition| *definition == cards::SWAMP)
                .expect("the Swamp is on offer")
        } else {
            offered
                .iter()
                .position(|definition| *definition == cards::LIGHTNING_BOLT)
                .expect("the Bolt is on offer")
        }
    });
    let exiled = game.players[0].exile[0].id;

    game.start_next_turn();
    game.start_next_turn();

    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::PlayLand { card, .. } if *card == exiled)),
        "two turns later the card is simply exiled",
    );
}

/// A library too short for the look still does as much as it can.
#[test]
fn two_cards_fill_the_first_two_destinations() {
    let (mut game, iteration) = staged(&[cards::LIGHTNING_BOLT, cards::ISLAND]);
    game.players[0].library.retain(|card| {
        card.definition == cards::LIGHTNING_BOLT || card.definition == cards::ISLAND
    });

    cast_iteration(&mut game, iteration, |_, offered| {
        offered
            .iter()
            .position(|definition| *definition == cards::LIGHTNING_BOLT)
            .unwrap_or(0)
    });

    assert_eq!(hand(&game), vec![cards::LIGHTNING_BOLT], "one went to hand");
    assert_eq!(
        library_from_bottom(&game),
        vec![cards::ISLAND],
        "and the other underneath",
    );
    assert!(exile(&game).is_empty(), "with nothing left to exile");
}

/// "If it's a land, you can't play it unless you have a land play
/// available." The permission is to play the card, not to break the rule
/// about how many lands a turn holds.
#[test]
fn an_exiled_land_still_wants_a_land_drop() {
    let (mut game, iteration) = staged(&[cards::ISLAND, cards::SWAMP, cards::LIGHTNING_BOLT]);
    game.players[0].lands_played_this_turn = 1;

    cast_iteration(&mut game, iteration, |prompt, offered| {
        let wanted = if prompt.contains("hand") {
            cards::SWAMP
        } else {
            cards::LIGHTNING_BOLT
        };
        offered
            .iter()
            .position(|definition| *definition == wanted)
            .expect("it is on offer")
    });

    assert_eq!(exile(&game), vec![cards::ISLAND]);
    let exiled = game.players[0].exile[0].id;
    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::PlayLand { card, .. } if *card == exiled)),
        "the land drop was already spent this turn",
    );

    game.players[0].lands_played_this_turn = 0;

    assert!(
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::PlayLand { card, .. } if *card == exiled)),
        "and with one available the same card is playable",
    );
}

/// "You must still pay all costs." The Iteration spends the two mana it was
/// cast with, and the Bolt it exiled waits for a red one.
#[test]
fn an_exiled_spell_still_costs_what_it_costs() {
    let (mut game, iteration) = staged(&[cards::ISLAND, cards::SWAMP, cards::LIGHTNING_BOLT]);

    cast_iteration(&mut game, iteration, |prompt, offered| {
        let wanted = if prompt.contains("hand") {
            cards::SWAMP
        } else {
            cards::ISLAND
        };
        offered
            .iter()
            .position(|definition| *definition == wanted)
            .expect("it is on offer")
    });

    assert_eq!(exile(&game), vec![cards::LIGHTNING_BOLT]);
    let exiled = game.players[0].exile[0].id;
    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == exiled)),
        "permission to play it is not a discount on it",
    );

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);

    assert!(
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == exiled)),
        "one red mana is what it was waiting for",
    );
}

/// One card is enough for the first instruction and nothing after it. The
/// two later destinations have no card to take, and the one card still
/// reaches the hand: the moves are chained behind the second question, so a
/// question with nothing to ask must not strand them.
#[test]
fn a_single_card_still_reaches_the_hand() {
    let (mut game, iteration) = staged(&[cards::LIGHTNING_BOLT]);
    game.players[0]
        .library
        .retain(|card| card.definition == cards::LIGHTNING_BOLT);

    cast_iteration(&mut game, iteration, |_, _| 0);

    assert_eq!(hand(&game), vec![cards::LIGHTNING_BOLT], "it went to hand");
    assert!(game.players[0].library.is_empty(), "and nothing is left");
    assert!(exile(&game).is_empty(), "with nothing to exile");
}

/// An empty library asks nothing and does nothing, and the Iteration still
/// resolves rather than hanging on a question it cannot pose.
#[test]
fn an_empty_library_asks_nothing() {
    let (mut game, iteration) = staged(&[]);
    game.players[0].library.clear();

    let asked = cast_iteration(&mut game, iteration, |_, _| 0);

    assert!(asked.is_empty(), "there was nothing to look at");
    assert!(hand(&game).is_empty());
    assert!(exile(&game).is_empty());
    assert!(game.stack.is_empty(), "and the spell finished resolving");
}
