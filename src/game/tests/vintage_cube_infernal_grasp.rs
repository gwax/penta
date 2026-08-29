//! Infernal Grasp: two mana answers anything, and the two life is the whole
//! of what it asks in return.

use super::*;

/// Player One holding a Grasp with two mana up, and `theirs` on the other
/// side of the board.
fn staged(theirs: CardDefinitionId) -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let target = game
        .put_onto_battlefield(PlayerId::Two, theirs)
        .expect("cataloged");
    drain_pending(&mut game);
    let spell = game
        .build_zone(PlayerId::One, &[cards::INFERNAL_GRASP])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let held = spell.id;
    game.players[0].hand.push(spell);
    game.players[0].life = 20;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 2);
    game.turns_started = [2, 1];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, held, target)
}

/// Casts the Grasp at `target` and lets it resolve.
fn grasp(game: &mut Game, held: GameObjectId, target: GameObjectId) {
    let cast =
        game.legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| match action {
                Action::CastSpell { card, choices, .. } => {
                    *card == held
                        && choices.targets().iter().any(|selection| {
                            selection.targets().contains(&Target::Permanent(target))
                        })
                }
                _ => false,
            })
            .expect("it can point at that creature");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(game);
}

fn alive(game: &Game, id: GameObjectId) -> bool {
    game.battlefield
        .iter()
        .any(|permanent| permanent.card.id == id)
}

/// The ordinary case: the creature dies and you are two life poorer.
#[test]
fn it_destroys_a_creature_for_two_life() {
    let (mut game, held, bears) = staged(cards::GRIZZLY_BEARS);

    grasp(&mut game, held, bears);

    assert!(!alive(&game, bears), "the creature was destroyed");
    assert!(
        game.players[1]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS),
        "and it is in its owner's graveyard",
    );
    assert_eq!(game.players[0].life, 18, "two life is the price");
}

/// No restriction on what it answers: colour, size and type are all beside
/// the point.
#[test]
fn nothing_about_the_creature_protects_it() {
    let (mut game, held, titan) = staged(cards::GRAVE_TITAN);

    grasp(&mut game, held, titan);

    assert!(!alive(&game, titan), "a black 6/6 dies to it all the same");
    assert_eq!(game.players[0].life, 18);
}

/// The life is part of the resolution, not a condition on it: an
/// indestructible creature survives and the two life is still paid.
#[test]
fn an_indestructible_creature_survives_but_the_life_is_still_paid() {
    let (mut game, held, blightsteel) = staged(cards::BLIGHTSTEEL_COLOSSUS);

    grasp(&mut game, held, blightsteel);

    assert!(alive(&game, blightsteel), "indestructible is not destroyed");
    assert_eq!(
        game.players[0].life, 18,
        "the Grasp resolved, so the life went anyway",
    );
}

/// "Target creature" and nothing more: your own is a legal target too.
#[test]
fn it_can_point_at_your_own_creature() {
    let (mut game, held, _theirs) = staged(cards::GRIZZLY_BEARS);
    let yours = game
        .put_onto_battlefield(PlayerId::One, cards::SAVANNAH_LIONS)
        .expect("cataloged");
    drain_pending(&mut game);

    grasp(&mut game, held, yours);

    assert!(!alive(&game, yours), "your own creature is a legal target");
    assert_eq!(game.players[0].life, 18);
}

/// It is an instant: it answers a creature on their turn as readily as on
/// yours.
#[test]
fn it_answers_on_their_turn() {
    let (mut game, held, bears) = staged(cards::GRIZZLY_BEARS);
    game.active_player = PlayerId::Two;
    game.priority = PlayerId::One;

    grasp(&mut game, held, bears);

    assert!(!alive(&game, bears));
    assert_eq!(game.players[0].life, 18);
}

/// The other side of the life clause: it is part of the resolution, so a
/// Grasp whose target is gone before it resolves is countered by the game
/// rules and costs nothing at all (CR 608.2b).
#[test]
fn a_grasp_with_nothing_to_kill_costs_no_life() {
    let (mut game, held, target) = staged(cards::GRIZZLY_BEARS);
    let cast =
        game.legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| match action {
                Action::CastSpell { card, choices, .. } => {
                    *card == held
                        && choices.targets().iter().any(|selection| {
                            selection.targets().contains(&Target::Permanent(target))
                        })
                }
                _ => false,
            })
            .expect("it can point at their creature");
    game.apply(PlayerId::One, cast).expect("it is cast");

    // Answered while the Grasp is on the stack.
    game.move_permanents_to_graveyard(&[target]);
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert_eq!(
        game.players[0].life, 20,
        "no target, no resolution, and so no life",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::INFERNAL_GRASP),
        "the Grasp is spent all the same",
    );
}

/// Nothing checks whether you can afford it: two life at two life is the
/// price of the last creature you answer.
#[test]
fn it_will_take_your_last_two_life() {
    let (mut game, held, target) = staged(cards::SERRA_ANGEL);
    game.players[0].life = 2;

    grasp(&mut game, held, target);
    game.check_state_based_actions();

    assert!(!alive(&game, target), "the Angel died");
    assert_eq!(game.players[0].life, 0);
    assert!(game.result.is_some(), "and so did you");
}
