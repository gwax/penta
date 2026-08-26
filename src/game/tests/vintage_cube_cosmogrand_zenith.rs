//! Cosmogrand Zenith: the second spell each turn pays again, and the choice
//! is between going wider and going taller.

use super::*;

/// The Zenith on the battlefield under Player One with a bear beside him,
/// and `hand` in hand with mana to spare.
fn staged(hand: &[CardDefinitionId]) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let zenith = game
        .put_onto_battlefield(PlayerId::One, cards::COSMOGRAND_ZENITH)
        .expect("cataloged");
    game.put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
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
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    for color in [ManaColor::Red, ManaColor::Green, ManaColor::Colorless] {
        game.add_unrestricted_mana(PlayerId::One, color, 6);
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, zenith, held)
}

/// Answers decisions, preferring mode `mode` where one is offered.
fn settle(game: &mut Game, mode: usize) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .get(mode)
                .or_else(|| decision.options.first())
                .map(|option| vec![option.id])
                .unwrap_or_default();
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
    drain_pending(game);
}

fn cast(game: &mut Game, held: GameObjectId, mode: usize) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == held))
        .expect("it is castable");
    game.apply(PlayerId::One, action).expect("it is cast");
    settle(game, mode);
}

fn soldiers(game: &Game) -> usize {
    game.battlefield
        .iter()
        .filter(|permanent| game.effective_subtypes(permanent).contains(&"Soldier"))
        .filter(|permanent| permanent.card.definition != cards::COSMOGRAND_ZENITH)
        .count()
}

fn counters_on(game: &Game, id: GameObjectId) -> u16 {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .map_or(0, |permanent| {
            permanent.counters(CounterKind::PlusOnePlusOne)
        })
}

/// The first spell does nothing; the second offers the choice.
#[test]
fn the_second_spell_makes_two_soldiers() {
    let (mut game, _, held) = staged(&[cards::LIGHTNING_BOLT, cards::GIANT_GROWTH]);

    cast(&mut game, held[0], 0);
    assert_eq!(soldiers(&game), 0, "the first spell pays nothing");

    cast(&mut game, held[1], 0);

    assert_eq!(soldiers(&game), 2, "and the second makes two Soldiers");
}

/// The other mode grows everything you have, the Zenith included.
#[test]
fn the_other_mode_grows_the_board() {
    let (mut game, zenith, held) = staged(&[cards::LIGHTNING_BOLT, cards::GIANT_GROWTH]);
    let bears = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::GRIZZLY_BEARS)
        .expect("the bear is there")
        .card
        .id;

    cast(&mut game, held[0], 1);
    cast(&mut game, held[1], 1);

    assert_eq!(counters_on(&game, zenith), 1, "the Zenith counts himself");
    assert_eq!(counters_on(&game, bears), 1, "and the bear beside him");
    assert_eq!(soldiers(&game), 0, "the other mode was not taken");
}

/// "Your second spell each turn": exactly the second, so the third pays
/// nothing.
#[test]
fn the_third_spell_pays_nothing() {
    let (mut game, _, held) = staged(&[
        cards::LIGHTNING_BOLT,
        cards::GIANT_GROWTH,
        cards::LIGHTNING_BOLT,
    ]);

    cast(&mut game, held[0], 0);
    cast(&mut game, held[1], 0);
    assert_eq!(soldiers(&game), 2);

    cast(&mut game, held[2], 0);

    assert_eq!(soldiers(&game), 2, "the third is not the second");
}

/// The count is per turn, and it is yours: their spells are not counted.
#[test]
fn their_spells_are_not_yours() {
    let (mut game, _, held) = staged(&[cards::GIANT_GROWTH]);
    let theirs = game
        .build_zone(PlayerId::Two, &[cards::LIGHTNING_BOLT])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let their_bolt = theirs.id;
    game.players[1].hand.push(theirs);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Red, 1);
    game.priority = PlayerId::Two;
    let cast_theirs = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == their_bolt))
        .expect("they can cast it");
    game.apply(PlayerId::Two, cast_theirs).expect("it is cast");
    settle(&mut game, 0);
    game.priority = PlayerId::One;

    cast(&mut game, held[0], 0);

    assert_eq!(
        soldiers(&game),
        0,
        "their Bolt did not make your Growth the second spell",
    );
}
