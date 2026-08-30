//! The Endstone: a card for everything you do, and the ten life handed back
//! every end step that makes the seven mana payable.

use super::*;

/// The Endstone on the battlefield under Player One, with a stocked library
/// and `hand` in hand.
fn staged(hand: &[CardDefinitionId]) -> (Game, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for index in 0..10 {
        game.players[0]
            .library
            .push(card(119_000 + index, cards::ISLAND, PlayerId::One));
    }
    game.put_onto_battlefield(PlayerId::One, cards::THE_ENDSTONE)
        .expect("cataloged");
    drain_pending(&mut game);
    let mut held = Vec::new();
    for definition in hand {
        let card = game
            .build_zone(PlayerId::One, &[*definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        held.push(card.id);
        game.players[0].hand.push(card);
    }
    game.players[0].lands_played_this_turn = 0;
    game.players[0].life = 20;
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, held)
}

fn settle(game: &mut Game) {
    for _ in 0..24 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    drain_pending(game);
}

/// A land drop draws a card.
#[test]
fn playing_a_land_draws() {
    let (mut game, held) = staged(&[cards::FOREST]);
    let library = game.players[0].library.len();

    let play = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::PlayLand { card, .. } if *card == held[0]))
        .expect("the land drop is available");
    game.apply(PlayerId::One, play).expect("it is played");
    settle(&mut game);

    assert_eq!(game.players[0].library.len(), library - 1, "one card drawn");
    assert_eq!(game.players[0].hand.len(), 1);
}

/// So does casting a spell.
#[test]
fn casting_a_spell_draws() {
    let (mut game, held) = staged(&[cards::LIGHTNING_BOLT]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    let library = game.players[0].library.len();

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == held[0]))
        .expect("one red mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game);

    assert_eq!(game.players[0].library.len(), library - 1, "one card drawn");
}

/// A land an effect puts onto the battlefield was never played, so nothing
/// is drawn for it.
#[test]
fn a_land_put_onto_the_battlefield_draws_nothing() {
    let (mut game, _) = staged(&[]);
    let library = game.players[0].library.len();

    game.put_onto_battlefield(PlayerId::One, cards::FOREST)
        .expect("cataloged");
    settle(&mut game);

    assert_eq!(game.players[0].library.len(), library, "nothing was played");
}

/// Their land drop is not yours.
#[test]
fn their_land_drop_draws_nothing() {
    let (mut game, _) = staged(&[]);
    let theirs = game
        .build_zone(PlayerId::Two, &[cards::FOREST])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let held = theirs.id;
    game.players[1].hand.push(theirs);
    game.players[1].lands_played_this_turn = 0;
    game.active_player = PlayerId::Two;
    game.priority = PlayerId::Two;
    let library = game.players[0].library.len();

    let play = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| matches!(action, Action::PlayLand { card, .. } if *card == held))
        .expect("their land drop is available");
    game.apply(PlayerId::Two, play).expect("it is played");
    settle(&mut game);

    assert_eq!(
        game.players[0].library.len(),
        library,
        "\"you play\" is you"
    );
}

/// The end step sets your life to ten, from above and from below alike.
#[test]
fn the_end_step_sets_life_to_half_the_start() {
    for before in [3, 20, 40] {
        let (mut game, _) = staged(&[]);
        game.players[0].life = before;
        game.step = Step::End;
        game.begin_step_triggers();
        settle(&mut game);

        assert_eq!(
            game.players[0].life, 10,
            "twenty to start makes ten either way, from {before}",
        );
    }
}

/// It is your end step and not theirs.
#[test]
fn their_end_step_leaves_your_life_alone() {
    let (mut game, _) = staged(&[]);
    game.players[0].life = 3;
    game.active_player = PlayerId::Two;
    game.priority = PlayerId::Two;
    game.step = Step::End;
    game.begin_step_triggers();
    settle(&mut game);

    assert_eq!(game.players[0].life, 3, "their end step is not yours");
}

/// "The Endstone's first ability resolves before the spell that caused it to
/// trigger." The card is drawn while that spell is still waiting.
#[test]
fn the_draw_resolves_before_the_spell_that_caused_it() {
    let (mut game, held) = staged(&[cards::LIGHTNING_BOLT]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    let library = game.players[0].library.len();

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == held[0]))
        .expect("one red mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    for _ in 0..4 {
        if game.stack.len() == 2 {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }

    assert_eq!(
        game.stack.last().map(|object| object.kind),
        Some(StackObjectKind::TriggeredAbility),
        "the trigger went on top of the spell that caused it",
    );
    pass_priority_pair(&mut game);

    assert_eq!(
        game.players[0].library.len(),
        library - 1,
        "and it resolved first, drawing the card",
    );
    assert!(
        game.stack
            .iter()
            .any(|object| object.kind == StackObjectKind::Spell),
        "while the Bolt is still on the stack, unresolved",
    );
}

/// "It resolves even if that spell is countered or otherwise leaves the
/// stack without resolving." The draw is not part of the spell.
#[test]
fn the_draw_happens_even_when_the_spell_is_countered() {
    let (mut game, held) = staged(&[cards::LIGHTNING_BOLT]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    game.players[1].hand.clear();
    let counterspell = card(119_500, cards::COUNTERSPELL, PlayerId::Two);
    let counterspell_id = counterspell.id;
    game.players[1].hand.push(counterspell);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Blue, 2);
    let library = game.players[0].library.len();

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == held[0]))
        .expect("one red mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    game.apply(PlayerId::One, Action::PassPriority)
        .expect("the window is theirs");
    let bolt = game
        .stack
        .iter()
        .find(|object| object.kind == StackObjectKind::Spell)
        .expect("the Bolt is under the trigger")
        .id;
    game.apply(
        PlayerId::Two,
        cast_action(counterspell_id, vec![Target::Spell(bolt)], Vec::new(), 0),
    )
    .expect("the Counterspell answers it");
    settle(&mut game);

    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::LIGHTNING_BOLT),
        "the Bolt was countered",
    );
    assert_eq!(
        game.players[0].library.len(),
        library - 1,
        "and the draw it caused happened all the same",
    );
}
