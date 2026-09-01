//! Mana Drain: a Counterspell that pays you what it took.
//!
//! When it pays -- your next main phase, this turn's or the next one's -- is
//! checked where the counterspells live as a family. What this adds is how
//! much, and when it pays nothing at all.

use super::*;

/// Player Two casting `theirs` for `x`, with a Drain in Player One's hand.
fn staged(theirs: CardDefinitionId, x: u16) -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::Two.index()].hand.clear();
    let spell = card(108_000, theirs, PlayerId::Two);
    let spell_id = spell.id;
    game.players[PlayerId::Two.index()].hand.push(spell);
    let drain = card(108_001, cards::MANA_DRAIN, PlayerId::One);
    let drain_id = drain.id;
    game.players[PlayerId::One.index()].hand.push(drain);
    for color in ManaColor::COLORS {
        game.add_unrestricted_mana(PlayerId::Two, color, 4);
    }
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 2);
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::Two;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::Two;

    let cast = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if *card == spell_id && choices.x() == x)
        })
        .expect("their spell is castable");
    game.apply(PlayerId::Two, cast).expect("it is cast");
    game.apply(PlayerId::Two, Action::PassPriority)
        .expect("they pass with it waiting");
    (game, drain_id, spell_id)
}

/// The spell object their card became, which is what the Drain names.
fn spell_on_stack(game: &Game) -> GameObjectId {
    game.stack
        .iter()
        .next()
        .expect("their spell is on the stack")
        .id
}

/// Runs to Player One's next precombat main phase and lets the delayed
/// trigger pay out.
fn to_your_main_phase(game: &mut Game) {
    for _ in 0..4 {
        game.finish_cleanup();
        game.start_next_turn();
        if game.active_player == PlayerId::One {
            break;
        }
    }
    assert_eq!(game.active_player, PlayerId::One, "your own turn");
    game.step = Step::Draw;
    game.advance_step();
    game.finish_rules_procedure();
    pass_priority_pair(game);
}

/// "An amount of {C} equal to that spell's mana value": a spell cast for X
/// has the X in its mana value, so a Ballista cast for three is six.
#[test]
fn the_payout_counts_the_x_the_spell_was_cast_for() {
    let (mut game, drain, _spell) = staged(cards::WALKING_BALLISTA, 3);
    let target = spell_on_stack(&game);

    game.apply(
        PlayerId::One,
        cast_action(drain, vec![Target::Spell(target)], Vec::new(), 0),
    )
    .expect("a spell on the stack is what it names");
    pass_priority_pair(&mut game);
    assert!(
        game.battlefield.is_empty(),
        "the Ballista was countered on the way down",
    );

    to_your_main_phase(&mut game);
    println!(
        "POOL {:?} STEP {:?} ACTIVE {:?} GRAVE {:?}",
        game.players[PlayerId::One.index()].mana_pool,
        game.step,
        game.active_player,
        game.players[PlayerId::Two.index()]
            .graveyard
            .iter()
            .map(|c| c.definition)
            .collect::<Vec<_>>()
    );

    assert_eq!(
        game.players[PlayerId::One.index()].mana_pool.colorless,
        6,
        "a doubled X cast for three is a mana value of six",
    );
}

/// "If the target spell is an illegal target by the time Mana Drain tries to
/// resolve, Mana Drain doesn't resolve. You don't add mana." Their own
/// second spell taking the first off the stack leaves the Drain with
/// nothing.
#[test]
fn a_drain_whose_target_is_gone_pays_nothing() {
    let (mut game, drain, _spell) = staged(cards::GRIZZLY_BEARS, 0);
    let target = spell_on_stack(&game);
    game.apply(
        PlayerId::One,
        cast_action(drain, vec![Target::Spell(target)], Vec::new(), 0),
    )
    .expect("their Bears is what it names");

    // They answer their own spell first, which takes the Drain's target away.
    let counter = card(108_100, cards::COUNTERSPELL, PlayerId::Two);
    let counter_id = counter.id;
    game.players[PlayerId::Two.index()].hand.push(counter);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Blue, 2);
    game.priority = PlayerId::Two;
    let answer = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == counter_id
                    && choices
                        .iter_targets()
                        .any(|chosen| *chosen == Target::Spell(target))
            }
            _ => false,
        })
        .expect("a spell may be countered by its own caster");
    game.apply(PlayerId::Two, answer).expect("it is cast");
    for _ in 0..8 {
        if game.stack.iter().count() == 0 {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }

    to_your_main_phase(&mut game);

    assert_eq!(
        game.players[PlayerId::One.index()].mana_pool.colorless,
        0,
        "the Drain never resolved, so there is nothing to add",
    );
    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::MANA_DRAIN),
        "and the Drain is spent all the same",
    );
}
