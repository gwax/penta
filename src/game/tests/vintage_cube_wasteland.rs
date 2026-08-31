//! Wasteland: a colourless land that trades itself for any land that is not
//! a basic.
//!
//! Which lands it may name is settled where the family is tested; what this
//! file adds is the rest of the card -- the mana it makes, the fact that it
//! reaches your own side, that the cost takes it out of its own target list,
//! and that a land which has become a creature is a land still.

use super::*;

/// Player One with a Wasteland out since last turn.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    let wasteland = game
        .put_onto_battlefield(PlayerId::One, cards::WASTELAND)
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
    (game, wasteland)
}

/// Every land the Wasteland is offering to destroy right now.
fn targets(game: &Game, wasteland: GameObjectId) -> Vec<GameObjectId> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } if source == wasteland => Some(
                targets
                    .iter()
                    .flat_map(crate::casting::TargetSelection::targets)
                    .filter_map(|target| match target {
                        Target::Permanent(id) => Some(*id),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .flatten()
        .collect()
}

fn blow_up(game: &mut Game, wasteland: GameObjectId, victim: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } => {
                *source == wasteland
                    && targets
                        .iter()
                        .flat_map(crate::casting::TargetSelection::targets)
                        .any(|target| *target == Target::Permanent(victim))
            }
            _ => false,
        })
        .expect("that land is on offer");
    game.apply(PlayerId::One, action).expect("it activates");
    pass_priority_pair(game);
    drain_pending(game);
    game.check_state_based_actions();
}

/// The half that gets played every game it is drawn: a colourless mana.
#[test]
fn it_taps_for_one_colorless() {
    let (mut game, wasteland) = staged();

    game.apply(
        PlayerId::One,
        Action::ActivateManaAbility {
            source: wasteland,
            ability: mana_ability_for(&game, wasteland, ManaColor::Colorless),
            color: ManaColor::Colorless,
            counters_removed: None,
            cost_object: None,
            combination: None,
            triggered_mana: None,
        },
    )
    .expect("it taps for colourless");

    assert_eq!(game.players[PlayerId::One.index()].mana_pool.colorless, 1);
    assert_eq!(
        game.players[PlayerId::One.index()].mana_pool.total(),
        1,
        "one mana and only one",
    );
}

/// "Target nonbasic land" names no controller, so your own Cradle is as good
/// a target as theirs. The Wasteland is a nonbasic land too, and targets are
/// chosen before costs are paid, so it is on its own list -- what stops that
/// line is resolution, where the target it named is already in the
/// graveyard.
#[test]
fn it_reaches_either_side_and_even_itself() {
    let (mut game, wasteland) = staged();
    let mine = game
        .put_onto_battlefield(PlayerId::One, cards::GAEAS_CRADLE)
        .expect("cataloged");
    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::BADLANDS)
        .expect("cataloged");
    game.put_onto_battlefield(PlayerId::Two, cards::MOUNTAIN)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    let offered = targets(&game, wasteland);
    assert!(
        offered.contains(&mine),
        "your own nonbasic is a land it may name"
    );
    assert!(offered.contains(&theirs), "and so is theirs");
    assert!(
        offered.contains(&wasteland),
        "and so is the Wasteland, which is nonbasic and still standing",
    );
    assert!(
        !offered.contains(
            &game
                .battlefield
                .iter()
                .find(|permanent| permanent.card.definition == cards::MOUNTAIN)
                .expect("their Mountain is there")
                .card
                .id
        ),
        "the Mountain is basic: {offered:?}",
    );

    blow_up(&mut game, wasteland, mine);
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == mine),
        "your own Cradle is what you chose to give up",
    );
}

/// Naming itself is legal and worth nothing: the cost has taken it off the
/// battlefield by the time the ability resolves, so the ability is left with
/// no legal target and everything else survives.
#[test]
fn naming_itself_destroys_nothing_else() {
    let (mut game, wasteland) = staged();
    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::BADLANDS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    blow_up(&mut game, wasteland, wasteland);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == wasteland),
        "it was sacrificed to pay for the ability",
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == theirs),
        "and the land it did not name is untouched",
    );
}

/// A land that has made itself a creature is a land still, and a nonbasic
/// one: the Wasteland answers a manland that a creature answer would have to
/// catch mid-turn.
#[test]
fn an_animated_manland_is_still_a_land() {
    let (mut game, wasteland) = staged();
    let pit = game
        .put_onto_battlefield(PlayerId::Two, cards::CREEPING_TAR_PIT)
        .expect("cataloged");
    drain_pending(&mut game);
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == pit)
    {
        permanent.tapped = false;
        permanent.entered_controller_turn = 0;
    }
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Blue, 1);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Black, 1);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Colorless, 1);
    game.priority = PlayerId::Two;
    let animate = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == pit))
        .expect("they can wake it up");
    game.apply(PlayerId::Two, animate).expect("it activates");
    drain_pending(&mut game);
    assert!(
        game.permanent_types(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == pit)
                .expect("it is there")
        )
        .is_some_and(super::CardTypeSet::is_creature),
        "it is a creature now",
    );
    game.priority = PlayerId::One;

    blow_up(&mut game, wasteland, pit);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == pit),
        "and a creature land is a land the Wasteland may destroy",
    );
}
