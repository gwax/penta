//! Stormchaser's Talent: a Class that starts as an Otter, buys back a spell
//! at level 2, and makes an Otter per spell at level 3.

use super::*;

fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    let talent = game
        .put_onto_battlefield(PlayerId::One, cards::STORMCHASERS_TALENT)
        .expect("cataloged");
    settle(&mut game);
    game.priority = PlayerId::One;
    (game, talent)
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
                .take(decision.minimum.max(1))
                .map(|option| option.id)
                .collect();
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the decision accepts what it offered");
            continue;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            return;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            return;
        }
    }
}

fn otters(game: &Game) -> usize {
    game.battlefield
        .iter()
        .filter(|permanent| {
            is_token_with(
                permanent,
                token_with_prowess(tokens::creature(
                    &["Otter"],
                    &[ManaColor::Blue, ManaColor::Red],
                    1,
                    1,
                )),
            )
        })
        .count()
}

fn level_counters(game: &Game, talent: GameObjectId) -> u16 {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == talent)
        .map_or(0, |permanent| {
            permanent.counters(CounterKind::named("level"))
        })
}

/// Every way of levelling the Class that is on offer right now.
fn level_ups(game: &Game, talent: GameObjectId) -> Vec<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == talent),
        )
        .collect()
}

/// Puts an instant in Player One's hand and casts it, targeting nothing.
fn cast_a_cantrip(game: &mut Game, id: u32) {
    let spell = card(30_000 + id, cards::ANCESTRAL_RECALL, PlayerId::One);
    let spell_id = spell.id;
    game.players[0].hand.push(spell);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    game.priority = PlayerId::One;
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == spell_id))
        .expect("one blue buys an Ancestral Recall");
    game.apply(PlayerId::One, cast).expect("it is castable");
    settle(game);
}

/// A Class enters at level 1 with an Otter and no counters.
#[test]
fn it_enters_at_level_one_with_an_otter() {
    let (game, talent) = staged();

    assert_eq!(otters(&game), 1, "the entry made one");
    assert_eq!(level_counters(&game, talent), 0, "level 1 is no counters");
}

/// Level 2 is one counter, and reaching it returns a spell from the
/// graveyard.
#[test]
fn level_two_buys_back_a_spell() {
    let (mut game, talent) = staged();
    let bolt = game
        .build_zone(PlayerId::One, &[cards::LIGHTNING_BOLT])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    game.players[0].graveyard.push(bolt);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 4);

    let level = level_ups(&game, talent)
        .into_iter()
        .next()
        .expect("four mana buys level 2");
    game.apply(PlayerId::One, level).expect("it levels up");
    settle(&mut game);

    assert_eq!(level_counters(&game, talent), 1, "level 2 is one counter");
    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::LIGHTNING_BOLT),
        "and the Bolt came back",
    );
}

/// Below level 3 a cantrip makes nothing.
#[test]
fn casting_a_spell_below_level_three_makes_no_otter() {
    let (mut game, _) = staged();
    cast_a_cantrip(&mut game, 1);

    assert_eq!(otters(&game), 1, "still just the entry Otter");
}

/// At level 3 every instant or sorcery is another Otter.
#[test]
fn level_three_makes_an_otter_per_spell() {
    let (mut game, talent) = staged();
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == talent)
    {
        permanent.set_counters(CounterKind::named("level"), 2);
    }

    cast_a_cantrip(&mut game, 1);
    assert_eq!(otters(&game), 2, "one more");
    cast_a_cantrip(&mut game, 2);
    assert_eq!(otters(&game), 3, "and another");
}

/// "You can't activate the first level ability of a Class unless that Class
/// is level 1. Similarly, you can't activate the second level ability unless
/// that Class is level 2." One level is on offer at a time, and a Class at
/// the top has nothing left to buy.
#[test]
fn each_level_is_sold_only_from_the_one_below_it() {
    let (mut game, talent) = staged();
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 10);
    assert_eq!(
        level_ups(&game, talent).len(),
        1,
        "at level 1 only level 2 is for sale",
    );

    for (counters, expected, note) in [
        (1, 1, "at level 2 only level 3 is"),
        (2, 0, "and a Class at level 3 has nothing left to buy"),
    ] {
        if let Some(permanent) = game
            .battlefield
            .iter_mut()
            .find(|permanent| permanent.card.id == talent)
        {
            permanent.set_counters(CounterKind::named("level"), counters);
        }
        assert_eq!(level_ups(&game, talent).len(), expected, "{note}");
    }
}

/// The Otter has prowess: a noncreature spell pumps it.
#[test]
fn the_otter_has_prowess() {
    let (mut game, _) = staged();
    let otter = game
        .battlefield
        .iter()
        .find(|permanent| {
            is_token_with(
                permanent,
                token_with_prowess(tokens::creature(
                    &["Otter"],
                    &[ManaColor::Blue, ManaColor::Red],
                    1,
                    1,
                )),
            )
        })
        .expect("the entry made one")
        .card
        .id;
    let power_of = |game: &Game| {
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == otter)
            .and_then(|permanent| game.power(permanent))
    };
    assert_eq!(power_of(&game), Some(1));

    cast_a_cantrip(&mut game, 1);

    assert_eq!(power_of(&game), Some(2), "+1/+1 until end of turn");
}

/// Level 3 is bought from level 2 and not before, so the climb is two
/// activations -- and the level-2 clause fires on the way, which is the
/// buyback the deck is paying for.
#[test]
fn the_climb_to_three_goes_through_two() {
    let (mut game, talent) = staged();
    let bolt = game
        .build_zone(PlayerId::One, &[cards::LIGHTNING_BOLT])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    game.players[0].graveyard.push(bolt);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 10);

    let first = level_ups(&game, talent)
        .into_iter()
        .next()
        .expect("level 2 is what a level-1 Class may buy");
    game.apply(PlayerId::One, first).expect("it levels up");
    settle(&mut game);
    assert_eq!(level_counters(&game, talent), 1, "level 2 is one counter");
    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::LIGHTNING_BOLT),
        "and its clause bought the Bolt back",
    );

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 10);
    let second = level_ups(&game, talent)
        .into_iter()
        .next()
        .expect("and now level 3 is on offer");
    game.apply(PlayerId::One, second)
        .expect("it levels up again");
    settle(&mut game);

    assert_eq!(level_counters(&game, talent), 2, "level 3 is two counters");
}

/// "The level 3 class ability resolves before the spell that caused it to
/// trigger. It resolves even if that spell is countered." The Otter is paid
/// for by the casting rather than by the spell working.
#[test]
fn the_otter_arrives_even_if_the_spell_is_countered() {
    let (mut game, talent) = staged();
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == talent)
    {
        permanent.set_counters(CounterKind::named("level"), 2);
    }
    let before = otters(&game);

    let spell = card(30_500, cards::ANCESTRAL_RECALL, PlayerId::One);
    let spell_id = spell.id;
    game.players[0].hand.push(spell);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    let counter = card(30_501, cards::COUNTERSPELL, PlayerId::Two);
    let counter_id = counter.id;
    game.players[1].hand.push(counter);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Blue, 2);
    game.priority = PlayerId::One;

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == spell_id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Player(PlayerId::One))
            }
            _ => false,
        })
        .expect("one blue buys an Ancestral Recall");
    game.apply(PlayerId::One, cast).expect("it is castable");

    // The Otter trigger is put on the stack above the Recall, and anything
    // it asks is answered before either player has a window.
    for _ in 0..8 {
        let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        else {
            break;
        };
        let options = decision
            .options
            .iter()
            .map(|option| option.id)
            .take(decision.minimum.max(1).min(decision.maximum))
            .collect();
        game.apply(
            decision.player,
            Action::ChooseDecision {
                decision: decision.id,
                options,
            },
        )
        .expect("the offered choice is legal");
    }
    let recall = game
        .stack
        .iter()
        .find(|object| object.presentation().card_definition() == Some(cards::ANCESTRAL_RECALL))
        .map(|object| object.id)
        .expect("the Recall is on the stack");
    game.priority = PlayerId::Two;
    game.apply(
        PlayerId::Two,
        cast_action(counter_id, vec![Target::Spell(recall)], Vec::new(), 0),
    )
    .expect("they answer it");
    settle(&mut game);

    assert_eq!(
        otters(&game),
        before + 1,
        "the trigger resolved under the spell it was cast for",
    );
    assert_eq!(
        game.players[0].hand.len(),
        0,
        "and the Recall drew nothing, having been countered",
    );
}
