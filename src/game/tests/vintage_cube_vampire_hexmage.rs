//! Vampire Hexmage: a two-mana first striker that answers a planeswalker
//! outright, because loyalty is a counter like any other.

use super::*;

/// The Hexmage out, with something to point it at.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    let hexmage = game
        .put_onto_battlefield(PlayerId::One, cards::VAMPIRE_HEXMAGE)
        .expect("cataloged");
    drain_pending(&mut game);
    game.step = Step::PrecombatMain;
    game.active_player = PlayerId::One;
    game.priority = PlayerId::One;
    (game, hexmage)
}

fn activation(game: &Game, hexmage: GameObjectId, target: GameObjectId) -> Option<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } => {
                *source == hexmage
                    && targets
                        .iter()
                        .flat_map(|selection| selection.targets().to_vec())
                        .any(|chosen| chosen == Target::Permanent(target))
            }
            _ => false,
        })
}

/// It has first strike.
#[test]
fn it_has_first_strike() {
    let (game, hexmage) = staged();
    let permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == hexmage)
        .expect("it is there");

    assert!(game.permanent_has_executable_keyword(permanent, KeywordAbility::FirstStrike));
}

/// Loyalty is a counter, so a planeswalker loses all of it and dies.
#[test]
fn it_kills_a_planeswalker() {
    let (mut game, hexmage) = staged();
    let jace = game
        .put_onto_battlefield(PlayerId::Two, cards::JACE_THE_MIND_SCULPTOR)
        .expect("cataloged");
    drain_pending(&mut game);
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == jace),
        "he starts with loyalty",
    );

    let action = activation(&game, hexmage, jace).expect("a sacrifice pays for it");
    game.apply(PlayerId::One, action).expect("it activates");
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != jace),
        "no loyalty is no planeswalker",
    );
}

/// Every kind comes off, not just one.
#[test]
fn it_removes_every_kind_of_counter() {
    let (mut game, hexmage) = staged();
    let bears = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == bears)
    {
        permanent.set_counters(CounterKind::PlusOnePlusOne, 3);
        permanent.set_counters(CounterKind::Flying, 1);
    }

    let action = activation(&game, hexmage, bears).expect("a sacrifice pays for it");
    game.apply(PlayerId::One, action).expect("it activates");
    drain_pending(&mut game);

    let stripped = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == bears)
        .expect("a 2/2 with nothing on it survives");
    assert_eq!(stripped.counters(CounterKind::PlusOnePlusOne), 0);
    assert_eq!(stripped.counters(CounterKind::Flying), 0);
    assert!(!game.permanent_has_executable_keyword(stripped, KeywordAbility::Flying));
    assert_eq!(game.power(stripped), Some(2));
}

/// Sacrificing itself is the cost, so the Hexmage is gone either way.
#[test]
fn activating_it_sacrifices_it() {
    let (mut game, hexmage) = staged();
    let bears = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);

    let action = activation(&game, hexmage, bears).expect("a sacrifice pays for it");
    game.apply(PlayerId::One, action).expect("it activates");
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != hexmage),
    );
}

/// "Target permanent": it can also point at your own.
#[test]
fn it_can_name_your_own_permanent() {
    let (mut game, hexmage) = staged();
    let yours = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);

    assert!(activation(&game, hexmage, yours).is_some());
}

/// "Target permanent", and a permanent is a thing on the battlefield: a card
/// waiting in exile with counters on it -- a suspended card is the ordinary
/// case -- is not one, however many counters it is carrying.
#[test]
fn it_cannot_reach_counters_outside_the_battlefield() {
    let (mut game, hexmage) = staged();
    let mut waiting = card(66_000, cards::SERRA_ANGEL, PlayerId::One);
    let waiting_id = waiting.id;
    waiting.add_counters(CounterKind::named("time"), 3);
    game.players[0].exile.push(waiting);

    assert!(
        !game.legal_actions(PlayerId::One).iter().any(|action| {
            matches!(action, Action::ActivateAbility { source, targets, .. }
            if *source == hexmage
                && targets.iter().any(|selection| {
                    selection.targets().iter().any(|chosen| {
                        matches!(
                            chosen,
                            Target::Permanent(id) | Target::Card(id) if *id == waiting_id
                        )
                    })
                }))
        }),
        "an exiled card is not a permanent to name",
    );

    // And with nothing else out, the ability has nobody to point at but the
    // Hexmage itself.
    let others = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == hexmage),
        )
        .count();
    assert_eq!(others, 1, "only the Hexmage itself is on the battlefield");
}
