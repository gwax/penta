//! Thespian's Stage: two mana and a tap to become whatever land is worth
//! being, for as long as it is on the battlefield.

use super::*;

/// The Stage and `others` on the battlefield since last turn, with two
/// colorless up for the copy.
fn staged(others: &[CardDefinitionId]) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    let stage = game
        .put_onto_battlefield(PlayerId::One, cards::THESPIANS_STAGE)
        .expect("cataloged");
    let mut ids = Vec::new();
    for definition in others {
        ids.push(
            game.put_onto_battlefield(PlayerId::One, *definition)
                .expect("cataloged"),
        );
    }
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 3);
    (game, stage, ids)
}

fn permanent(game: &Game, id: GameObjectId) -> &Permanent {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("it is on the battlefield")
}

/// Copies `target`, letting the ability resolve.
fn copy_onto(game: &mut Game, stage: GameObjectId, target: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } => {
                *source == stage
                    && targets
                        .iter()
                        .flat_map(crate::casting::TargetSelection::targets)
                        .any(|selected| *selected == Target::Permanent(target))
            }
            _ => false,
        })
        .expect("the copy ability is offered for that land");
    game.apply(PlayerId::One, action).expect("it activates");
    drain_pending(game);
    game.check_state_based_actions();
}

/// "A land's copiable values are those printed on it, as modified by other
/// copy effects ... if you copy a land that is also a creature because of a
/// temporary effect, Thespian's Stage will become just the 'unanimated'
/// land." A Mishra's Factory swinging as a 2/2 is still only a Factory to
/// copy.
#[test]
fn copying_an_animated_land_leaves_the_animation_behind() {
    let (mut game, stage, others) = staged(&[cards::MISHRA_S_FACTORY]);
    let factory = others[0];

    let animate = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == factory),
        )
        .expect("one mana animates it");
    game.apply(PlayerId::One, animate).expect("it activates");
    drain_pending(&mut game);
    assert!(
        game.permanent_types(permanent(&game, factory))
            .is_some_and(CardTypeSet::is_creature),
        "the Factory is a 2/2 right now",
    );

    copy_onto(&mut game, stage, factory);

    assert_eq!(
        game.object_card_name(stage).as_deref(),
        Some("Mishra's Factory"),
        "it copied the Factory",
    );
    let copied = permanent(&game, stage);
    assert!(
        !game
            .permanent_types(copied)
            .is_some_and(CardTypeSet::is_creature),
        "but not the animation, which was never a copiable value",
    );
    assert!(
        game.permanent_types(permanent(&game, factory))
            .is_some_and(CardTypeSet::is_creature),
        "and the original is still animated",
    );
}

/// "Thespian's Stage doesn't become untapped when it becomes a copy, even if
/// the target land is untapped." The tap was the cost, and copying an
/// untapped land does not refund it.
#[test]
fn it_stays_tapped_after_copying_an_untapped_land() {
    let (mut game, stage, others) = staged(&[cards::FOREST]);
    let forest = others[0];
    assert!(!permanent(&game, forest).tapped, "the Forest is untapped");

    copy_onto(&mut game, stage, forest);

    assert!(
        permanent(&game, stage).tapped,
        "the tap paid for the copy and stays paid",
    );
    assert!(
        !permanent(&game, forest).tapped,
        "however untapped the thing it copied is",
    );
}

/// "Except it has this ability": the copy keeps the copying ability and so
/// can be pointed at something else later, but it no longer has the Stage's
/// own mana ability -- it has the copied land's instead.
#[test]
fn the_copy_may_copy_again_and_taps_for_what_it_became() {
    let (mut game, stage, others) = staged(&[cards::FOREST, cards::MOUNTAIN]);
    let (forest, mountain) = (others[0], others[1]);

    copy_onto(&mut game, stage, forest);
    assert!(
        game.legal_actions(PlayerId::One).iter().any(|action| {
            matches!(action, Action::ActivateManaAbility { source, color, .. }
                if *source == stage && *color == ManaColor::Green)
        }) || permanent(&game, stage).tapped,
        "it is a Forest now, tapped from paying for the copy",
    );

    // Untap it and it taps for green rather than for colorless.
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == stage)
        .expect("it is there")
        .tapped = false;
    let colors = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::ActivateManaAbility { source, color, .. } if source == stage => Some(color),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        colors,
        vec![ManaColor::Green],
        "green and nothing else: the Stage's own colorless tap is gone",
    );

    // And the copy ability came along, so it may become the Mountain later.
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);
    copy_onto(&mut game, stage, mountain);
    assert!(
        game.effective_land_types(permanent(&game, stage))[BasicLandType::Mountain.index()],
        "a second copy overwrites the first",
    );
    assert!(
        !game.effective_land_types(permanent(&game, stage))[BasicLandType::Forest.index()],
        "and replaces it rather than stacking with it",
    );
}
