//! Nettlecyst: living weapon on a count of everything artificial or
//! enchanted you have out, the Equipment included.

use super::*;

/// The Germ living weapon made, which is the only token Nettlecyst creates.
fn germ(game: &Game) -> &Permanent {
    game.battlefield
        .iter()
        .find(|permanent| {
            is_token_with(
                permanent,
                tokens::creature(&["Phyrexian", "Germ"], &[ManaColor::Black], 0, 0),
            )
        })
        .expect("living weapon made a Germ and the count kept it alive")
}

/// Activates `source`'s targeted ability at `target`.
fn activate_at(game: &mut Game, source: GameObjectId, target: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source: actual,
                targets,
                ..
            } => {
                *actual == source
                    && targets
                        .iter()
                        .flat_map(crate::casting::TargetSelection::targets)
                        .any(|selected| *selected == Target::Permanent(target))
            }
            _ => false,
        })
        .expect("the ability is offered for that permanent");
    game.apply(PlayerId::One, action)
        .expect("the ability activates");
    drain_pending(game);
    game.check_state_based_actions();
}

/// "If a permanent you control is both an artifact and an enchantment, count
/// it only once when determining the bonus from an equipped Nettlecyst."
/// Ashnod's Transmogrant makes the Courser an artifact without taking the
/// enchantment away, and the Courser is still worth exactly one.
#[test]
fn a_permanent_that_is_both_counts_once() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.put_onto_battlefield(PlayerId::One, cards::NETTLECYST)
        .expect("cataloged");
    drain_pending(&mut game);
    let courser = game
        .put_onto_battlefield(PlayerId::One, cards::COURSER_OF_KRUPHIX)
        .expect("cataloged");
    let transmogrant = game
        .put_onto_battlefield(PlayerId::One, cards::ASHNODS_TRANSMOGRANT)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.check_state_based_actions();

    let before = game.power(germ(&game));
    assert_eq!(
        before,
        Some(3),
        "the Equipment, the enchantment creature, and the Transmogrant",
    );

    activate_at(&mut game, transmogrant, courser);

    let courser = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == courser)
        .expect("it only gained a type");
    let types = game
        .permanent_types(courser)
        .expect("it is on the battlefield");
    assert!(
        types.contains(CardType::Artifact) && types.contains(CardType::Enchantment),
        "it is both now: {types:?}",
    );
    assert_eq!(
        game.power(germ(&game)),
        Some(2),
        "the Equipment and the Courser once, not the Courser twice",
    );
}
