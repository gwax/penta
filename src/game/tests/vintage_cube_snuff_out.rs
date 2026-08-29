//! Snuff Out: four life and a Swamp for a creature, and what it kills is
//! killed past regeneration.

use super::*;

/// Player One holding a Snuff Out with `land` on the battlefield and an
/// Uthden Troll across the table.
fn staged(land: CardDefinitionId) -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.put_onto_battlefield(PlayerId::One, land)
        .expect("cataloged");
    drain_pending(&mut game);
    let troll = creature(130_000, cards::UTHDEN_TROLL, PlayerId::Two);
    let troll_id = troll.card.id;
    game.battlefield.push(troll);
    let snuff = card(130_100, cards::SNUFF_OUT, PlayerId::One);
    let snuff_id = snuff.id;
    game.players[0].hand.push(snuff);
    game.players[0].life = 20;
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, snuff_id, troll_id)
}

fn free_cast(game: &Game, snuff: GameObjectId, target: GameObjectId) -> Option<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == snuff
                    && choices.costs().alternative().is_some()
                    && choices
                        .targets()
                        .iter()
                        .any(|selection| selection.targets().contains(&Target::Permanent(target)))
            }
            _ => false,
        })
}

/// "If you control a Swamp" reads the land type, so a Watery Grave is a
/// Swamp and a Forest is not.
#[test]
fn a_dual_with_the_swamp_type_pays_for_it() {
    let (game, snuff, troll) = staged(cards::WATERY_GRAVE);
    assert!(
        free_cast(&game, snuff, troll).is_some(),
        "the Grave is a Swamp, whatever else it is",
    );

    let (game, snuff, troll) = staged(cards::FOREST);
    assert!(
        free_cast(&game, snuff, troll).is_none(),
        "and a Forest is not one",
    );
}

/// "It can't be regenerated": a Troll with a shield up dies anyway.
#[test]
fn regeneration_does_not_save_what_it_names() {
    let (mut game, snuff, troll) = staged(cards::SWAMP);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Red, 1);
    game.priority = PlayerId::Two;
    let regenerate = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == troll))
        .expect("one red buys a shield");
    game.apply(PlayerId::Two, regenerate).expect("it activates");
    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == troll && permanent.regeneration_shields > 0),
        "the shield is up",
    );

    game.priority = PlayerId::One;
    let cast = free_cast(&game, snuff, troll).expect("a Swamp and the life pay for it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);
    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    game.check_state_based_actions();

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == troll),
        "the shield was no answer to it",
    );
    assert_eq!(game.players[0].life, 16, "and the four life was paid");
}
