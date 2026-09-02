//! Goblin Bombardment: a free sacrifice outlet that points its payment
//! anywhere on the table.

use super::*;

/// The Bombardment on the battlefield with `mine` beside it and `theirs`
/// across the table.
fn staged(
    mine: &[CardDefinitionId],
    theirs: &[CardDefinitionId],
) -> (Game, GameObjectId, Vec<GameObjectId>, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    let bombardment = game
        .put_onto_battlefield(PlayerId::One, cards::GOBLIN_BOMBARDMENT)
        .expect("cataloged");
    let mut ours = Vec::new();
    let mut theirs_ids = Vec::new();
    for (player, definitions, ids) in [
        (PlayerId::One, mine, &mut ours),
        (PlayerId::Two, theirs, &mut theirs_ids),
    ] {
        for definition in definitions {
            ids.push(
                game.put_onto_battlefield(player, *definition)
                    .expect("cataloged"),
            );
        }
    }
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.check_state_based_actions();
    (game, bombardment, ours, theirs_ids)
}

/// Every activation the Bombardment is offering right now.
fn offers(game: &Game, bombardment: GameObjectId) -> Vec<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(|action| {
            matches!(action, Action::ActivateAbility { source, .. } if *source == bombardment)
        })
        .collect()
}

/// Throws a creature at `target` and lets the damage happen.
fn throw_at(game: &mut Game, bombardment: GameObjectId, target: Target) {
    let action = offers(game, bombardment)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility { targets, .. } => targets
                .iter()
                .flat_map(crate::casting::TargetSelection::targets)
                .any(|selected| *selected == target),
            _ => false,
        })
        .unwrap_or_else(|| panic!("{target:?} is one of the offered targets"));
    game.apply(PlayerId::One, action).expect("it activates");
    drain_pending(game);
    game.check_state_based_actions();
}

/// "Any target" is more than the opponent's face: a one-toughness creature
/// dies to it and a planeswalker loses a loyalty.
#[test]
fn it_points_at_creatures_and_planeswalkers_too() {
    let (mut game, bombardment, _mine, theirs) = staged(
        &[cards::GRIZZLY_BEARS, cards::GRIZZLY_BEARS],
        &[cards::SAVANNAH_LIONS, cards::DACK_FAYDEN],
    );
    let (lions, dack) = (theirs[0], theirs[1]);

    throw_at(&mut game, bombardment, Target::Permanent(lions));
    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != lions),
        "one damage is all a 2/1 can take",
    );

    let loyalty = |game: &Game| {
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == dack)
            .expect("he is there")
            .counters(CounterKind::Loyalty)
    };
    let before = loyalty(&game);
    throw_at(&mut game, bombardment, Target::Permanent(dack));
    assert_eq!(
        loyalty(&game),
        before - 1,
        "and a planeswalker pays for it in loyalty",
    );
}

/// "Sacrifice a creature" means one of yours. Their board is no help, so a
/// Bombardment with nothing of its own to throw is not offering anything.
#[test]
fn only_your_own_creatures_pay_for_it() {
    let (game, bombardment, _mine, _theirs) = staged(&[], &[cards::GRIZZLY_BEARS]);

    assert!(
        offers(&game, bombardment).is_empty(),
        "their bear is not yours to sacrifice",
    );
}

/// Nothing but a tap-free cost, so it fires as often as there are creatures
/// to feed it -- three bears is three damage in one main phase, and the
/// fourth activation is not on offer.
#[test]
fn it_fires_once_for_every_creature_you_can_spare() {
    let (mut game, bombardment, mine, _theirs) = staged(
        &[
            cards::GRIZZLY_BEARS,
            cards::GRIZZLY_BEARS,
            cards::GRIZZLY_BEARS,
        ],
        &[],
    );
    assert_eq!(mine.len(), 3, "three to spend");
    let life = game.players[PlayerId::Two.index()].life;

    for round in 1..=3 {
        assert!(
            !offers(&game, bombardment).is_empty(),
            "a creature is still standing before round {round}",
        );
        throw_at(&mut game, bombardment, Target::Player(PlayerId::Two));
        assert_eq!(
            game.players[PlayerId::Two.index()].life,
            life - round,
            "one point per bear after round {round}",
        );
    }

    assert!(
        offers(&game, bombardment).is_empty(),
        "and with the board empty there is nothing left to throw",
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == bombardment),
        "the Bombardment itself is no creature and stays put",
    );
}
