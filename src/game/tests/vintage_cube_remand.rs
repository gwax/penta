//! Remand: two mana that buys a turn and replaces itself. What it answers
//! comes back, so it is tempo rather than an answer.

use super::*;

/// Player Two casting a spell, Player One holding the Remand.
fn staged(theirs: CardDefinitionId) -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    game.players[1].hand.clear();
    game.players[1].graveyard.clear();
    for (index, definition) in [cards::MOUNTAIN, cards::FOREST].into_iter().enumerate() {
        let id = 283_000 + u32::try_from(index).expect("two cards");
        game.players[0]
            .library
            .push(card(id, definition, PlayerId::One));
    }
    let remand = game
        .build_zone(PlayerId::One, &[cards::REMAND])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let remand_id = remand.id;
    game.players[0].hand.push(remand);
    let spell = game
        .build_zone(PlayerId::Two, &[theirs])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let spell_id = spell.id;
    game.players[1].hand.push(spell);
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::Two;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::Two;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 2);
    for color in ManaColor::COLORS {
        game.add_unrestricted_mana(PlayerId::Two, color, 5);
    }
    (game, remand_id, spell_id)
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

/// Player Two casts their spell; Player One answers it with the Remand.
fn cast_and_answer(game: &mut Game, remand: GameObjectId, spell: GameObjectId) {
    let cast = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == spell))
        .expect("they have the mana");
    game.apply(PlayerId::Two, cast)
        .expect("their spell is cast");
    for _ in 0..4 {
        if game.priority == PlayerId::One {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    let answer = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == remand))
        .expect("two mana answers it");
    game.apply(PlayerId::One, answer).expect("it is cast");
    settle(game);
}

/// The countered spell goes back to its owner's hand, not their graveyard.
#[test]
fn the_countered_card_goes_back_to_hand() {
    let (mut game, remand, spell) = staged(cards::SERRA_ANGEL);

    cast_and_answer(&mut game, remand, spell);

    assert!(game.battlefield.is_empty(), "the creature never resolved");
    assert_eq!(
        game.players[1]
            .hand
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::SERRA_ANGEL],
        "and they have it back",
    );
    assert!(game.players[1].graveyard.is_empty());
}

/// And Remand replaces itself.
#[test]
fn it_draws_a_card() {
    let (mut game, remand, spell) = staged(cards::SERRA_ANGEL);

    cast_and_answer(&mut game, remand, spell);

    assert_eq!(game.players[0].hand.len(), 1, "one card drawn");
    assert_eq!(game.players[0].library.len(), 1);
}

/// The Remand itself is an ordinary spell and goes to its own graveyard.
#[test]
fn the_remand_goes_to_its_own_graveyard() {
    let (mut game, remand, spell) = staged(cards::SERRA_ANGEL);

    cast_and_answer(&mut game, remand, spell);

    assert_eq!(
        game.players[0]
            .graveyard
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::REMAND],
    );
}

/// It answers an instant the same way: what it names is any spell.
#[test]
fn it_answers_an_instant_too() {
    let (mut game, remand, spell) = staged(cards::LIGHTNING_BOLT);

    cast_and_answer(&mut game, remand, spell);

    assert_eq!(game.players[0].life, 20, "no damage was dealt");
    assert_eq!(
        game.players[1]
            .hand
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::LIGHTNING_BOLT],
    );
}

/// "Remand can target a spell that can't be countered. That spell won't be
/// countered or returned to its owner's hand, but you'll draw a card."
#[test]
fn it_still_draws_off_a_spell_it_cannot_counter() {
    let (mut game, remand, spell) = staged(cards::SUPREME_VERDICT);
    game.battlefield
        .push(creature(283_100, cards::GRIZZLY_BEARS, PlayerId::One));
    let library = game.players[0].library.len();

    cast_and_answer(&mut game, remand, spell);

    assert!(
        game.battlefield.is_empty(),
        "the Verdict resolved and swept the board",
    );
    assert!(
        game.players[1].hand.is_empty(),
        "it was not put back into their hand",
    );
    assert_eq!(
        game.players[1]
            .graveyard
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::SUPREME_VERDICT],
        "it went where a resolved sorcery goes",
    );
    assert_eq!(
        game.players[0].library.len(),
        library - 1,
        "and the Remand drew its card either way",
    );
}

/// "If you target a card that was cast with flashback with Remand, the card
/// will still be exiled." Flashback's own replacement is what moves it, and
/// it wins over "put it into its owner's hand instead".
#[test]
fn a_flashback_spell_is_exiled_rather_than_returned() {
    let (mut game, remand, _unused) = staged(cards::GRIZZLY_BEARS);
    game.players[1].hand.clear();
    let flashed = game
        .build_zone(PlayerId::Two, &[cards::FEELING_OF_DREAD])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let flashed_id = flashed.id;
    game.players[1].graveyard.push(flashed);

    cast_and_answer(&mut game, remand, flashed_id);

    assert!(
        game.players[1].hand.is_empty(),
        "the counter did not put it back in their hand",
    );
    assert!(
        game.players[1].graveyard.is_empty(),
        "and it did not stay in the graveyard either",
    );
    assert!(
        game.players[1]
            .exile
            .iter()
            .any(|card| card.definition == cards::FEELING_OF_DREAD),
        "flashback exiles what it was cast from wherever the spell ends up",
    );
}
