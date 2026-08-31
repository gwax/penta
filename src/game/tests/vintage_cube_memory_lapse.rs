//! Memory Lapse: two mana that buys a turn rather than a card.

use super::*;

/// Player Two holding a spell to cast, Player One holding the Lapse.
fn staged(theirs: CardDefinitionId) -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[1].hand.clear();
    game.players[1].library.clear();
    let lapse = game
        .build_zone(PlayerId::One, &[cards::MEMORY_LAPSE])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let lapse_id = lapse.id;
    game.players[0].hand.push(lapse);
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
        game.add_unrestricted_mana(PlayerId::Two, color, 4);
    }
    (game, lapse_id, spell_id)
}

fn settle(game: &mut Game) {
    for _ in 0..24 {
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

/// Player Two casts their spell; Player One answers it with the Lapse.
fn cast_and_answer(game: &mut Game, lapse: GameObjectId, spell: GameObjectId) {
    let cast = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == spell))
        .expect("they have the mana");
    game.apply(PlayerId::Two, cast).expect("it is cast");

    // The spell is on the stack; the window to answer it opens once
    // priority reaches the other player.
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
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == lapse))
        .expect("the Lapse can answer it");
    game.apply(PlayerId::One, answer).expect("it is cast");
    settle(game);
}

/// The spell is countered, and its card goes to the top of its owner's
/// library rather than to their graveyard.
#[test]
fn the_countered_card_goes_on_top_of_the_library() {
    let (mut game, lapse, spell) = staged(cards::SERRA_ANGEL);

    cast_and_answer(&mut game, lapse, spell);

    assert!(game.battlefield.is_empty(), "the creature never resolved");
    assert_eq!(
        game.players[1].library.last().map(|card| card.definition),
        Some(cards::SERRA_ANGEL),
        "on top of their library",
    );
    assert!(
        game.players[1].graveyard.is_empty(),
        "and not in their graveyard",
    );
}

/// The Lapse itself is an ordinary spell and goes to its own graveyard.
#[test]
fn the_lapse_goes_to_its_own_graveyard() {
    let (mut game, lapse, spell) = staged(cards::SERRA_ANGEL);

    cast_and_answer(&mut game, lapse, spell);

    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::MEMORY_LAPSE),
    );
}

/// It counters an instant the same way: what it names is any spell.
#[test]
fn it_counters_an_instant_too() {
    let (mut game, lapse, spell) = staged(cards::ANCESTRAL_RECALL);
    let hand = game.players[0].hand.len();

    cast_and_answer(&mut game, lapse, spell);

    assert_eq!(
        game.players[1].library.last().map(|card| card.definition),
        Some(cards::ANCESTRAL_RECALL),
    );
    assert_eq!(
        game.players[0].hand.len(),
        hand - 1,
        "and nobody drew from it",
    );
}

/// They draw it again next turn, which is the whole shape of the card: a
/// turn bought rather than a card taken.
#[test]
fn they_draw_it_again() {
    let (mut game, lapse, spell) = staged(cards::SERRA_ANGEL);
    cast_and_answer(&mut game, lapse, spell);
    assert!(game.players[1].hand.is_empty());

    game.draw_cards(PlayerId::Two, 1);

    assert!(
        game.players[1]
            .hand
            .iter()
            .any(|card| card.definition == cards::SERRA_ANGEL),
        "the same card, one turn later",
    );
}

/// "If the spell was cast using flashback, flashback will change the spell's
/// destination from its owner's library to exile." The Lapse's replacement
/// says library; flashback's says exile, and exile is where it goes.
#[test]
fn a_flashback_spell_is_exiled_rather_than_stacked() {
    let (mut game, lapse, _unused) = staged(cards::GRIZZLY_BEARS);
    game.players[1].hand.clear();
    let flashed = game
        .build_zone(PlayerId::Two, &[cards::FEELING_OF_DREAD])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let flashed_id = flashed.id;
    game.players[1].graveyard.push(flashed);

    cast_and_answer(&mut game, lapse, flashed_id);

    assert!(
        game.players[1].library.is_empty(),
        "it never reached the top of their library",
    );
    assert!(
        game.players[1].graveyard.is_empty(),
        "nor stayed in the graveyard it was cast from",
    );
    assert!(
        game.players[1]
            .exile
            .iter()
            .any(|card| card.definition == cards::FEELING_OF_DREAD),
        "flashback exiles it wherever the counter would have sent it",
    );
}

/// "Put it on top of its owner's library": the owner, not the player who
/// cast it. A spell cast off somebody else's card -- which is what Ragavan's
/// exile leaves on the stack -- goes home to the library it came from.
#[test]
fn the_card_goes_to_its_owners_library_and_not_its_casters() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[1].hand.clear();
    game.players[0].library.clear();
    game.players[1].library.clear();

    let mut borrowed = spell(77_000, cards::SERRA_ANGEL, PlayerId::One, 0);
    borrowed.card.owner = PlayerId::Two;
    let borrowed_id = borrowed.id;
    game.stack.push(borrowed);

    let lapse = card(77_001, cards::MEMORY_LAPSE, PlayerId::Two);
    let lapse_id = lapse.id;
    game.players[1].hand.push(lapse);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Blue, 2);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::Two;

    game.apply(
        PlayerId::Two,
        cast_action(lapse_id, vec![Target::Spell(borrowed_id)], Vec::new(), 0),
    )
    .expect("the Lapse answers it");
    settle(&mut game);

    assert!(
        game.players[0].library.is_empty(),
        "casting it does not make it yours",
    );
    assert_eq!(
        game.players[1].library.last().map(|card| card.definition),
        Some(cards::SERRA_ANGEL),
        "it is on top of the library of the player who owns it",
    );
    assert!(
        game.players[0].graveyard.is_empty() && game.players[1].graveyard.len() == 1,
        "and the only card in a graveyard is the Lapse itself",
    );
}

/// "If that spell is countered this way" is a condition and not a
/// certainty. A legendary spell paid for with Delighted Halfling mana cannot
/// be countered, so there is no counter for the replacement to redirect: it
/// resolves, and nothing goes anywhere near a library.
///
/// The game is built here rather than staged, because the fixture hands
/// Player Two a pool of every colour and what this test needs is a seat
/// whose only green is the Halfling's.
#[test]
fn a_spell_that_cannot_be_countered_is_not_stacked_either() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[1].hand.clear();
    game.players[1].library.clear();
    let lapse = card(76_400, cards::MEMORY_LAPSE, PlayerId::One);
    let lapse_id = lapse.id;
    game.players[0].hand.push(lapse);
    let tifa = card(76_401, cards::TIFA_LOCKHART, PlayerId::Two);
    let tifa_id = tifa.id;
    game.players[1].hand.push(tifa);
    let halfling = creature(76_402, cards::DELIGHTED_HALFLING, PlayerId::Two);
    let halfling_id = halfling.card.id;
    game.battlefield.push(halfling);
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::Two;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::Two;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 2);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Colorless, 1);
    game.apply(
        PlayerId::Two,
        Action::ActivateManaAbility {
            source: halfling_id,
            ability: mana_ability_for(&game, halfling_id, ManaColor::Green),
            color: ManaColor::Green,
            counters_removed: None,
            cost_object: None,
            combination: None,
            triggered_mana: None,
        },
    )
    .expect("the Halfling taps for a colour");
    let library = game.players[1].library.len();

    cast_and_answer(&mut game, lapse_id, tifa_id);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::TIFA_LOCKHART),
        "she resolved through it",
    );
    assert_eq!(
        game.players[1].library.len(),
        library,
        "and nothing was put on top of their library",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::MEMORY_LAPSE),
        "the Lapse is spent either way",
    );
}
