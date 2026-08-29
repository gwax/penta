//! Flame Slash: one mana for four damage, which is the best rate in the
//! format and pays for it in timing and reach.

use super::*;

/// Player One holding a Slash with the mana for it and a Serra Angel across
/// the table.
fn staged() -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let angel = game
        .put_onto_battlefield(PlayerId::Two, cards::SERRA_ANGEL)
        .expect("cataloged");
    drain_pending(&mut game);
    let slash = card(96_100, cards::FLAME_SLASH, PlayerId::One);
    let slash_id = slash.id;
    game.players[0].hand.push(slash);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, slash_id, angel)
}

fn castable(game: &Game, slash: GameObjectId) -> bool {
    game.legal_actions(PlayerId::One)
        .iter()
        .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == slash))
}

/// A sorcery waits for your own main phase, which is what separates it from
/// the instants that answer a creature mid-combat.
#[test]
fn it_waits_for_your_own_main_phase() {
    let (mut game, slash, _angel) = staged();
    assert!(castable(&game, slash), "your main phase, stack empty");

    game.step = Step::DeclareBlockers;
    assert!(!castable(&game, slash), "not in the middle of combat");

    game.step = Step::PrecombatMain;
    game.active_player = PlayerId::Two;
    assert!(!castable(&game, slash), "and not on their turn");
}

/// Four damage is damage: an indestructible creature takes it and stays,
/// however much of it there is.
#[test]
fn an_indestructible_creature_shrugs_it_off() {
    let (mut game, slash, _angel) = staged();
    let juggernaut = game
        .put_onto_battlefield(PlayerId::Two, cards::DARKSTEEL_JUGGERNAUT)
        .expect("cataloged");
    drain_pending(&mut game);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == slash
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(juggernaut))
            }
            _ => false,
        })
        .expect("it can point at the Juggernaut");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);
    game.check_state_based_actions();

    let still_there = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == juggernaut)
        .expect("indestructible is not a toughness");
    assert_eq!(
        still_there.damage, 4,
        "the damage is marked on it all the same",
    );
}
