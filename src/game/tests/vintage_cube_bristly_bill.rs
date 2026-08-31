//! Bristly Bill, Spine Sower: a counter for every land, and a doubling that
//! reads only your own side of the board.

use super::*;

/// Bill on the battlefield with `mine` beside him and `theirs` opposite,
/// each of those carrying `counters` +1/+1 counters.
fn staged(
    mine: &[CardDefinitionId],
    theirs: &[CardDefinitionId],
    counters: u16,
) -> (Game, GameObjectId, Vec<GameObjectId>, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let bill = creature(90_000, cards::BRISTLY_BILL_SPINE_SOWER, PlayerId::One);
    let bill_id = bill.card.id;
    game.battlefield.push(bill);
    let mut ours = Vec::new();
    let mut theirs_ids = Vec::new();
    for (index, definition) in mine.iter().enumerate() {
        let mut permanent = creature(
            90_100 + u32::try_from(index).expect("a handful"),
            *definition,
            PlayerId::One,
        );
        permanent.set_counters(CounterKind::PlusOnePlusOne, counters);
        ours.push(permanent.card.id);
        game.battlefield.push(permanent);
    }
    for (index, definition) in theirs.iter().enumerate() {
        let mut permanent = creature(
            90_200 + u32::try_from(index).expect("a handful"),
            *definition,
            PlayerId::Two,
        );
        permanent.set_counters(CounterKind::PlusOnePlusOne, counters);
        theirs_ids.push(permanent.card.id);
        game.battlefield.push(permanent);
    }
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, bill_id, ours, theirs_ids)
}

fn counters_on(game: &Game, id: GameObjectId) -> u16 {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("it is on the battlefield")
        .counters(CounterKind::PlusOnePlusOne)
}

/// "Double the number of +1/+1 counters on each creature you control."
/// Theirs is not one of yours, however many counters it is carrying.
#[test]
fn the_doubling_leaves_their_creatures_alone() {
    let (mut game, bill, mine, theirs) =
        staged(&[cards::GRIZZLY_BEARS], &[cards::GRIZZLY_BEARS], 2);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 2);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 3);

    let double = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == bill))
        .expect("five mana pays for it");
    game.apply(PlayerId::One, double).expect("it activates");
    drain_pending(&mut game);

    assert_eq!(counters_on(&game, mine[0]), 4, "two doubled is four");
    assert_eq!(
        counters_on(&game, theirs[0]),
        2,
        "and their Bears kept the two it had",
    );
}

/// "Put a +1/+1 counter on target creature" names no controller, so the
/// trigger may point at anything on the board -- their creature included.
#[test]
fn the_landfall_trigger_may_name_any_creature() {
    let (mut game, bill, mine, theirs) =
        staged(&[cards::GRIZZLY_BEARS], &[cards::SAVANNAH_LIONS], 0);
    game.put_onto_battlefield(PlayerId::One, cards::FOREST)
        .expect("cataloged");
    for _ in 0..8 {
        if !game.pending_decisions.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }

    let decision = game
        .pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("landfall asks for its target");
    let mut offered = decision
        .options
        .iter()
        .filter_map(|option| option.card.map(|(object, _)| object))
        .collect::<Vec<_>>();
    offered.sort_unstable();
    let mut expected = vec![bill, mine[0], theirs[0]];
    expected.sort_unstable();

    assert_eq!(
        offered, expected,
        "himself, his neighbour, and the creature across the table",
    );
}

/// The landfall trigger names its target as it goes on the stack, so a
/// creature answered underneath it takes the trigger with it: nothing else
/// on the board is grown instead.
#[test]
fn a_target_answered_underneath_the_trigger_takes_it_with_them() {
    let (mut game, bill, mine, _theirs) = staged(&[cards::GRIZZLY_BEARS], &[], 0);
    let bears = mine[0];
    game.put_onto_battlefield(PlayerId::One, cards::FOREST)
        .expect("cataloged");
    for _ in 0..8 {
        if !game.pending_decisions.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    let decision = game
        .pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("landfall asks for its target");
    let option = decision
        .options
        .iter()
        .find(|option| option.card.is_some_and(|(object, _)| object == bears))
        .map(|option| option.id)
        .expect("the Bears are on offer");
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![option],
        },
    )
    .expect("naming them is legal");

    game.move_permanents_to_graveyard(&[bears]);
    game.check_state_based_actions();
    drain_pending(&mut game);

    assert_eq!(
        counters_on(&game, bill),
        0,
        "the counter went with the creature it was promised to",
    );
}

/// The doubling has no restriction on how often it is used: five mana twice
/// over is four counters where there were one.
#[test]
fn the_doubling_may_be_activated_again() {
    let (mut game, bill, mine, _theirs) = staged(&[cards::GRIZZLY_BEARS], &[], 1);
    let bears = mine[0];

    for expected in [2, 4] {
        game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 2);
        game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 3);
        let double = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .find(
                |action| matches!(action, Action::ActivateAbility { source, .. } if *source == bill),
            )
            .expect("five mana pays for it");
        game.apply(PlayerId::One, double).expect("it activates");
        drain_pending(&mut game);

        assert_eq!(
            counters_on(&game, bears),
            expected,
            "doubling again doubles what the last one left",
        );
    }
}
