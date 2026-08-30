//! Enduring Innocence: a Glimmer that draws off small creatures once a turn
//! and gets back up as an enchantment the first time it dies.

use super::*;

fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    let innocence = game
        .put_onto_battlefield(PlayerId::One, cards::ENDURING_INNOCENCE)
        .expect("cataloged");
    drain_pending(&mut game);
    (game, innocence)
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

fn on_battlefield(game: &Game) -> Option<&Permanent> {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::ENDURING_INNOCENCE)
}

/// The types the Glimmer presents right now, after everything layered onto
/// it. The printed rules are not the answer once it has come back.
fn types_of(game: &Game) -> Option<CardTypeSet> {
    game.permanent_types(on_battlefield(game)?)
}

/// A small creature entering draws a card; a second one that turn does not.
#[test]
fn the_draw_happens_once_each_turn() {
    let (mut game, _) = staged();
    let before = game.players[0].hand.len();

    game.put_onto_battlefield(PlayerId::One, cards::MERFOLK_OF_THE_PEARL_TRIDENT)
        .expect("cataloged");
    settle(&mut game);
    assert_eq!(game.players[0].hand.len(), before + 1, "a 1/1 drew a card");

    game.put_onto_battlefield(PlayerId::One, cards::MERFOLK_OF_THE_PEARL_TRIDENT)
        .expect("cataloged");
    settle(&mut game);
    assert_eq!(
        game.players[0].hand.len(),
        before + 1,
        "the second one draws nothing this turn",
    );
}

/// A creature too big for the clause does not draw.
#[test]
fn a_bigger_creature_draws_nothing() {
    let (mut game, _) = staged();
    let before = game.players[0].hand.len();

    game.put_onto_battlefield(PlayerId::One, cards::SERRA_ANGEL)
        .expect("cataloged");
    settle(&mut game);

    assert_eq!(game.players[0].hand.len(), before, "power 4 is too much");
}

/// The Glimmer does not count itself, and an opponent's small creature is
/// not one of yours.
#[test]
fn it_counts_only_your_other_creatures() {
    let (mut game, _) = staged();
    let before = game.players[0].hand.len();

    game.put_onto_battlefield(PlayerId::Two, cards::MERFOLK_OF_THE_PEARL_TRIDENT)
        .expect("cataloged");
    settle(&mut game);

    assert_eq!(game.players[0].hand.len(), before);
}

/// It dies as a creature and stands back up as an enchantment.
#[test]
fn dying_returns_it_as_an_enchantment() {
    let (mut game, innocence) = staged();
    assert!(
        types_of(&game).is_some_and(
            |types| types.contains(CardType::Creature) && types.contains(CardType::Enchantment)
        ),
        "it starts as an enchantment creature",
    );

    game.destroy_permanent(innocence);
    settle(&mut game);

    let types = types_of(&game).expect("it came back");
    assert!(
        types.contains(CardType::Enchantment),
        "still an enchantment"
    );
    assert!(
        !types.contains(CardType::Creature),
        "and no longer a creature",
    );
}

/// The second death is the last one: an enchantment was not a creature, so
/// the clause finds nothing to bring back.
#[test]
fn the_second_death_is_permanent() {
    let (mut game, innocence) = staged();
    game.destroy_permanent(innocence);
    settle(&mut game);

    let returned = on_battlefield(&game).expect("it came back once").card.id;
    game.destroy_permanent(returned);
    settle(&mut game);

    assert!(
        on_battlefield(&game).is_none(),
        "an enchantment that dies stays dead",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::ENDURING_INNOCENCE),
        "and lies in the graveyard",
    );
}

/// Its ruling: "Sheep and Glimmer are both creature types. It won't have
/// those creature types when its last ability returns it to the battlefield
/// because it won't be a creature" (CR 205.1b).
#[test]
fn it_comes_back_without_its_creature_types() {
    let (mut game, innocence) = staged();
    let subtypes = |game: &Game| {
        let mut subtypes = on_battlefield(game)
            .map(|permanent| game.effective_subtypes(permanent).to_vec())
            .expect("it is there");
        subtypes.sort_unstable();
        subtypes
    };
    let printed = subtypes(&game);
    assert_eq!(
        printed,
        vec!["Glimmer", "Sheep"],
        "a Sheep Glimmer while it is a creature",
    );

    game.destroy_permanent(innocence);
    settle(&mut game);

    assert!(
        subtypes(&game).is_empty(),
        "and nothing at all once the creature is gone: they were creature types",
    );
}

/// "If a creature enters with counters, those counters are included." A
/// Walking Ballista is a 0/0 on paper and whatever X paid for on arrival, so
/// the size the Glimmer reads is the one the counters make.
#[test]
fn counters_an_arrival_brings_count_toward_the_two() {
    for (x, drew) in [(2, true), (3, false)] {
        let (mut game, _) = staged();
        game.players[0].hand.clear();
        let ballista = game
            .build_zone(PlayerId::One, &[cards::WALKING_BALLISTA])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        let ballista_id = ballista.id;
        game.players[0].hand.push(ballista);
        game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2 * x);
        game.active_player = PlayerId::One;
        game.step = Step::PrecombatMain;
        game.priority = PlayerId::One;

        let cast = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| match action {
                Action::CastSpell { card, choices, .. } => *card == ballista_id && choices.x() == x,
                _ => false,
            })
            .unwrap_or_else(|| panic!("a Ballista for X={x} is castable"));
        game.apply(PlayerId::One, cast).expect("it is cast");
        settle(&mut game);

        let arrival = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.definition == cards::WALKING_BALLISTA)
            .expect("it resolved");
        assert_eq!(
            game.power(arrival),
            Some(i16::try_from(x).expect("a small X")),
            "the counters are its whole body",
        );
        assert_eq!(
            game.players[0].hand.len(),
            usize::from(drew),
            "X={x} arrives as a {x}/{x}, which the clause {} read as small enough",
            if drew { "does" } else { "does not" },
        );
    }
}
