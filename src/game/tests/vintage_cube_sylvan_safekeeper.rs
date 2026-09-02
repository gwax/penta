//! Sylvan Safekeeper: a land a turn for shroud, as often as you have lands.
//!
//! That the land goes as a cost and that shroud stops both seats from
//! targeting is covered with the premodern permanents. What this adds is
//! whose creatures he reaches, what the ability answers, and how long the
//! shroud lasts.

use super::*;

/// The Safekeeper out with `lands` Forests and `others` beside him.
fn staged(lands: usize, others: &[CardDefinitionId]) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    let safekeeper = game
        .put_onto_battlefield(PlayerId::One, cards::SYLVAN_SAFEKEEPER)
        .expect("cataloged");
    let mut ids = Vec::new();
    for definition in others {
        ids.push(
            game.put_onto_battlefield(PlayerId::One, *definition)
                .expect("cataloged"),
        );
    }
    for index in 0..lands {
        game.battlefield.push(creature(
            99_500 + u32::try_from(index).expect("a few lands"),
            cards::FOREST,
            PlayerId::One,
        ));
    }
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turns_started = [5, 5];
    game.turn = 9;
    drain_pending(&mut game);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, safekeeper, ids)
}

/// Every creature the Safekeeper is offering to protect right now.
fn targets(game: &Game, safekeeper: GameObjectId) -> Vec<GameObjectId> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } if source == safekeeper => Some(
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

fn protect(game: &mut Game, safekeeper: GameObjectId, wanted: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } => {
                *source == safekeeper
                    && targets
                        .iter()
                        .flat_map(crate::casting::TargetSelection::targets)
                        .any(|target| *target == Target::Permanent(wanted))
            }
            _ => false,
        })
        .expect("that creature is on offer");
    game.apply(PlayerId::One, action).expect("it activates");
    drain_pending(game);
}

fn has_shroud(game: &Game, id: GameObjectId) -> bool {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .is_some_and(|permanent| {
            game.permanent_has_executable_keyword(permanent, KeywordAbility::Shroud)
        })
}

/// "Target creature you control" -- himself included, and nothing of theirs.
#[test]
fn he_protects_his_own_side_and_himself() {
    let (mut game, safekeeper, ids) = staged(1, &[cards::GRIZZLY_BEARS]);
    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::SAVANNAH_LIONS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    let offered = targets(&game, safekeeper);
    assert!(
        offered.contains(&safekeeper),
        "he may name himself, which is what he is for: {offered:?}",
    );
    assert!(offered.contains(&ids[0]), "and the creature beside him");
    assert!(!offered.contains(&theirs), "and nothing across the table");

    protect(&mut game, safekeeper, safekeeper);
    assert!(has_shroud(&game, safekeeper), "he took it himself");
}

/// The line the card is played for: shroud in response leaves their removal
/// with nothing legal to hit.
#[test]
fn shroud_in_response_makes_their_removal_fizzle() {
    let (mut game, safekeeper, ids) = staged(1, &[cards::SERRA_ANGEL]);
    let angel = ids[0];
    let bolt = game
        .build_zone(PlayerId::Two, &[cards::LIGHTNING_BOLT])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let bolt_id = bolt.id;
    game.players[PlayerId::Two.index()].hand.push(bolt);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Red, 1);
    game.priority = PlayerId::Two;

    let cast = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == bolt_id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(angel))
            }
            _ => false,
        })
        .expect("they can point it at the Angel");
    game.apply(PlayerId::Two, cast).expect("it is cast");
    game.priority = PlayerId::One;

    protect(&mut game, safekeeper, angel);
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == angel)
            .expect("she is still there")
            .damage,
        0,
        "the Bolt had nothing legal left to resolve against",
    );
}

/// Shroud lasts the turn and no longer, and the ability costs no mana: as
/// many lands as you can spare is as many creatures as you can cover.
#[test]
fn the_shroud_is_bought_by_the_land_and_lasts_the_turn() {
    let (mut game, safekeeper, ids) = staged(2, &[cards::GRIZZLY_BEARS, cards::SAVANNAH_LIONS]);

    protect(&mut game, safekeeper, ids[0]);
    protect(&mut game, safekeeper, ids[1]);
    assert!(
        has_shroud(&game, ids[0]) && has_shroud(&game, ids[1]),
        "two lands cover two creatures, with no mana between them",
    );
    assert!(
        targets(&game, safekeeper).is_empty(),
        "and with the lands gone there is nothing left to pay with",
    );

    let turn = game.turn;
    for _ in 0..60 {
        if game.turn > turn {
            break;
        }
        game.advance_step();
        drain_pending(&mut game);
    }

    assert!(
        !has_shroud(&game, ids[0]) && !has_shroud(&game, ids[1]),
        "until end of turn is over",
    );
}

/// Shroud is not protection: it shuts out every targeted spell, yours as
/// readily as theirs. A bear he has just covered is a bear your own Rancor
/// can no longer name -- and the Safekeeper cannot even name it again to
/// cover it twice.
#[test]
fn the_shroud_he_grants_shuts_your_own_spells_out_too() {
    let (mut game, safekeeper, ids) = staged(2, &[cards::GRIZZLY_BEARS]);
    let bears = ids[0];
    let rancor = card(99_600, cards::RANCOR, PlayerId::One);
    let rancor_id = rancor.id;
    game.players[PlayerId::One.index()].hand.push(rancor);
    game.players[PlayerId::One.index()].mana_pool.green = 1;

    let aims_at_the_bear = |game: &Game| {
        game.legal_actions(PlayerId::One)
            .into_iter()
            .any(|action| match action {
                Action::CastSpell { card, choices, .. } => {
                    card == rancor_id
                        && choices
                            .iter_targets()
                            .any(|target| *target == Target::Permanent(bears))
                }
                _ => false,
            })
    };
    assert!(
        aims_at_the_bear(&game),
        "an unguarded bear may be enchanted"
    );

    protect(&mut game, safekeeper, bears);
    assert!(has_shroud(&game, bears), "the bear is covered");

    assert!(
        !aims_at_the_bear(&game),
        "and your own Aura is a targeted spell like any other",
    );
    assert!(
        !targets(&game, safekeeper).contains(&bears),
        "so is his own ability: a covered creature cannot be covered again",
    );
}
