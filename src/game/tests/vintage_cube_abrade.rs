//! Abrade: two mana that is never dead, because the half a red deck wants
//! is whichever one the board is holding.

use super::*;

const BURN: usize = 0;
const SHATTER: usize = 1;

fn mode(index: usize) -> ModeId {
    ModeId::from_index(index).expect("one of the two")
}

/// Abrade in hand with the mana for it and `theirs` on their battlefield.
fn staged(theirs: &[CardDefinitionId]) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let mut ids = Vec::new();
    for definition in theirs {
        ids.push(
            game.put_onto_battlefield(PlayerId::Two, *definition)
                .expect("cataloged"),
        );
    }
    drain_pending(&mut game);
    let abrade = game
        .build_zone(PlayerId::One, &[cards::ABRADE])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = abrade.id;
    game.players[0].hand.push(abrade);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 2);
    game.turns_started = [2, 1];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, id, ids)
}

fn settle(game: &mut Game) {
    for _ in 0..16 {
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

/// Every mode Abrade is offering, with the target each would take.
fn casts(game: &Game, abrade: GameObjectId) -> Vec<(Vec<ModeId>, Vec<Target>)> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell {
                card: id, choices, ..
            } if id == abrade => Some((
                choices.modes().to_vec(),
                choices.iter_targets().copied().collect(),
            )),
            _ => None,
        })
        .collect()
}

fn cast_at(game: &mut Game, abrade: GameObjectId, index: usize, target: GameObjectId) {
    let wanted = [mode(index)];
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell {
                card: id, choices, ..
            } => {
                *id == abrade
                    && choices.modes() == wanted
                    && choices
                        .iter_targets()
                        .any(|chosen| *chosen == Target::Permanent(target))
            }
            _ => false,
        })
        .unwrap_or_else(|| panic!("mode {index} is castable at that permanent"));
    game.apply(PlayerId::One, action).expect("it is cast");
    settle(game);
}

fn on_battlefield(game: &Game, definition: CardDefinitionId) -> bool {
    game.battlefield
        .iter()
        .any(|permanent| permanent.card.definition == definition)
}

/// Three damage kills what a two-mana red spell is usually pointed at.
#[test]
fn the_first_mode_burns_a_creature() {
    let (mut game, abrade, ids) = staged(&[cards::GRIZZLY_BEARS]);

    cast_at(&mut game, abrade, BURN, ids[0]);

    assert!(!on_battlefield(&game, cards::GRIZZLY_BEARS), "it died");
}

/// A creature bigger than three survives it: the damage is damage rather
/// than destruction.
#[test]
fn a_larger_creature_survives() {
    let (mut game, abrade, ids) = staged(&[cards::SERRA_ANGEL]);

    cast_at(&mut game, abrade, BURN, ids[0]);

    assert!(
        on_battlefield(&game, cards::SERRA_ANGEL),
        "a 4/4 takes three and lives",
    );
}

/// The other half destroys an artifact outright, whatever its size.
#[test]
fn the_second_mode_destroys_an_artifact() {
    let (mut game, abrade, ids) = staged(&[cards::SOL_RING]);

    cast_at(&mut game, abrade, SHATTER, ids[0]);

    assert!(!on_battlefield(&game, cards::SOL_RING), "the Ring is gone");
}

/// An artifact creature is a legal thing for either half.
#[test]
fn an_artifact_creature_answers_to_both() {
    let (game, abrade, _) = staged(&[cards::ORNITHOPTER]);

    let offered = casts(&game, abrade);
    assert!(
        offered
            .iter()
            .any(|(modes, _)| modes.as_slice() == [mode(BURN)]),
        "it is a creature: {offered:?}",
    );
    assert!(
        offered
            .iter()
            .any(|(modes, _)| modes.as_slice() == [mode(SHATTER)]),
        "and an artifact: {offered:?}",
    );
}

/// One mode, and only the mode with something to point at.
#[test]
fn only_the_mode_with_a_legal_target_is_offered() {
    let (game, abrade, _) = staged(&[cards::GRIZZLY_BEARS]);

    let offered = casts(&game, abrade);
    assert!(!offered.is_empty(), "it is castable");
    assert!(
        offered
            .iter()
            .all(|(modes, _)| modes.as_slice() == [mode(BURN)]),
        "no artifact means no artifact mode: {offered:?}",
    );
}

/// With neither on the board it cannot be cast at all.
#[test]
fn an_empty_board_leaves_it_uncastable() {
    let (game, abrade, _) = staged(&[]);

    assert!(
        casts(&game, abrade).is_empty(),
        "both halves need something to point at",
    );
}
