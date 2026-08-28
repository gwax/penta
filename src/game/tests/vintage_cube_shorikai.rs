//! Shorikai, Genesis Engine: a looter that pays for its own crew.

use super::*;

/// Shorikai on the battlefield since last turn, with mana and a library.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for index in 0..12 {
        game.players[0]
            .library
            .push(card(72_000 + index, cards::ISLAND, PlayerId::One));
    }
    let shorikai = game
        .put_onto_battlefield(PlayerId::One, cards::SHORIKAI_GENESIS_ENGINE)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 6);
    (game, shorikai)
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

/// Activates the loot half, untapping Shorikai afterwards so it can be done
/// again.
fn loot(game: &mut Game, shorikai: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { source, ability: AbilityOrigin::Printed { ability, .. }, .. }
                if *source == shorikai && *ability == AbilityId(0))
        })
        .expect("one mana and a tap loots");
    game.apply(PlayerId::One, action).expect("it activates");
    settle(game);
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == shorikai)
    {
        permanent.tapped = false;
    }
}

fn pilots(game: &Game) -> Vec<&Permanent> {
    game.battlefield
        .iter()
        .filter(|permanent| permanent.card.definition == ObjectKind::Token)
        .collect()
}

fn crew_action(game: &Game, shorikai: GameObjectId) -> Option<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { source, ability: AbilityOrigin::Printed { ability, .. }, .. }
                if *source == shorikai && *ability == AbilityId(1))
        })
}

/// The loot draws two, bins one, and leaves a Pilot behind.
#[test]
fn it_loots_and_leaves_a_pilot() {
    let (mut game, shorikai) = staged();

    loot(&mut game, shorikai);

    assert_eq!(game.players[0].hand.len(), 1, "two drawn, one discarded");
    let pilots = pilots(&game);
    assert_eq!(pilots.len(), 1, "and one Pilot");
    assert_eq!(game.power(pilots[0]), Some(1), "which is a 1/1");
}

/// Each Pilot is worth three to a Vehicle rather than one, so three of them
/// crew an eight where nine ordinary power would be needed anyway -- and two
/// of them, worth six, are not enough.
#[test]
fn three_pilots_crew_the_eight() {
    let (mut game, shorikai) = staged();
    loot(&mut game, shorikai);
    loot(&mut game, shorikai);

    assert!(
        crew_action(&game, shorikai).is_none(),
        "two Pilots are six power between them",
    );

    loot(&mut game, shorikai);
    let crew = crew_action(&game, shorikai).expect("three Pilots are nine");
    game.apply(PlayerId::One, crew).expect("it crews");
    settle(&mut game);

    let vehicle = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == shorikai)
        .expect("it is there");
    assert!(
        game.permanent_types(vehicle)
            .is_some_and(|types| types.contains(CardType::Creature)),
        "the Vehicle is a creature now",
    );
    assert_eq!(game.power(vehicle), Some(8));
}

/// The bonus is for crewing only: the Pilot is still a 1/1 to everything
/// else that reads its power.
#[test]
fn the_pilot_is_still_a_one_one() {
    let (mut game, shorikai) = staged();
    loot(&mut game, shorikai);
    let pilot = pilots(&game)[0].card.id;

    let pilot = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == pilot)
        .expect("it is there");
    assert_eq!(game.power(pilot), Some(1), "power is untouched");
    assert_eq!(game.toughness(pilot), Some(1));
}

/// An ordinary creature contributes only what it is: four Bears are eight
/// power, and three are not.
#[test]
fn creatures_without_the_clause_pay_their_own_power() {
    let (mut game, shorikai) = staged();
    for _ in 0..3 {
        game.put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
            .expect("cataloged");
    }
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }

    assert!(
        crew_action(&game, shorikai).is_none(),
        "three Bears are six power",
    );

    game.put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }

    assert!(crew_action(&game, shorikai).is_some(), "and four are eight");
}

/// The loot activation right now, if it is on offer.
fn loot_action(game: &Game, shorikai: GameObjectId) -> Option<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { source, ability: AbilityOrigin::Printed { ability, .. }, .. }
                if *source == shorikai && *ability == AbilityId(0))
        })
}

/// A Vehicle is not a creature, so the tap in its cost is not a creature's
/// tap: Shorikai loots on the turn it lands. Crewing is what changes that --
/// what it leaves is a creature that has not been yours since the turn
/// began, and CR 302.6 holds its {T} down for the rest of the turn.
#[test]
fn it_loots_the_turn_it_lands_and_stops_once_it_is_crewed() {
    let (mut game, _old) = staged();
    game.battlefield.clear();
    let shorikai = game
        .put_onto_battlefield(PlayerId::One, cards::SHORIKAI_GENESIS_ENGINE)
        .expect("cataloged");
    drain_pending(&mut game);
    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == shorikai)
            .expect("it is there")
            .entered_controller_turn,
        game.turns_started[0],
        "it arrived this turn",
    );

    let loot = loot_action(&game, shorikai).expect("a Vehicle has no summoning sickness to have");
    game.apply(PlayerId::One, loot).expect("it activates");
    settle(&mut game);
    assert_eq!(pilots(&game).len(), 1, "and it left a Pilot behind");

    // Enough bodies to crew it, all of them just as new -- crew taps them as
    // a cost rather than with a {T} of their own, so their own newness does
    // not matter.
    for index in 0..4 {
        game.put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
            .unwrap_or_else(|error| panic!("bear {index}: {error}"));
    }
    drain_pending(&mut game);
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == shorikai)
    {
        permanent.tapped = false;
    }

    let crew = crew_action(&game, shorikai).expect("four Bears are eight power");
    game.apply(PlayerId::One, crew).expect("it crews");
    settle(&mut game);
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == shorikai)
    {
        permanent.tapped = false;
    }

    assert!(
        loot_action(&game, shorikai).is_none(),
        "now that it is a creature, its tap waits for your next turn",
    );
}
