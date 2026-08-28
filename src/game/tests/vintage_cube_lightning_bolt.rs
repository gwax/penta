//! Lightning Bolt: one mana for three damage at anything, and the reason a
//! four-toughness creature is a real line in the sand -- but only within one
//! turn, because marked damage does not carry over (CR 514.2).

use super::*;

/// Player One holding two Bolts and the mana for both, with a Serra Angel
/// across the table: 4/4, which is one more toughness than a Bolt.
fn staged() -> (Game, Vec<GameObjectId>, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let mut bolts = Vec::new();
    for _ in 0..2 {
        let bolt = game
            .build_zone(PlayerId::One, &[cards::LIGHTNING_BOLT])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        bolts.push(bolt.id);
        game.players[0].hand.push(bolt);
    }
    let angel = game
        .put_onto_battlefield(PlayerId::Two, cards::SERRA_ANGEL)
        .expect("cataloged");
    drain_pending(&mut game);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 2);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, bolts, angel)
}

fn bolt_at(game: &mut Game, bolt: GameObjectId, target: GameObjectId) {
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell {
                card: id, choices, ..
            } => {
                *id == bolt
                    && choices
                        .targets()
                        .iter()
                        .any(|selection| selection.targets() == [Target::Permanent(target)])
            }
            _ => false,
        })
        .expect("one red mana and anything on the battlefield");
    game.apply(PlayerId::One, cast).expect("it is cast");
    for _ in 0..8 {
        if game.stack.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    game.check_state_based_actions();
}

fn alive(game: &Game, id: GameObjectId) -> bool {
    game.battlefield
        .iter()
        .any(|permanent| permanent.card.id == id)
}

/// Three damage is three damage: an Angel with four toughness lives, with
/// the damage marked on it for the rest of the turn.
#[test]
fn one_bolt_leaves_a_four_toughness_creature_standing() {
    let (mut game, bolts, angel) = staged();
    bolt_at(&mut game, bolts[0], angel);

    assert!(alive(&game, angel), "three damage on four toughness");
    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == angel)
            .expect("still there")
            .damage,
        3,
        "and the damage stays marked while the turn lasts",
    );
}

/// Two of them in one turn add up, which is why the second Bolt is held.
#[test]
fn two_bolts_in_one_turn_add_up() {
    let (mut game, bolts, angel) = staged();
    bolt_at(&mut game, bolts[0], angel);
    bolt_at(&mut game, bolts[1], angel);

    assert!(!alive(&game, angel), "six damage on four toughness");
}

/// But damage is removed as the turn ends, so the same two Bolts a turn
/// apart do nothing at all -- the second one starts from zero.
#[test]
fn damage_does_not_carry_into_the_next_turn() {
    let (mut game, bolts, angel) = staged();
    bolt_at(&mut game, bolts[0], angel);
    game.cleanup();
    game.check_state_based_actions();
    // A fresh turn, with the mana for the second one.
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    assert!(alive(&game, angel), "it survived the first one");
    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == angel)
            .expect("still there")
            .damage,
        0,
        "and the cleanup step wiped what was marked",
    );

    bolt_at(&mut game, bolts[1], angel);
    assert!(
        alive(&game, angel),
        "so the second Bolt is three damage on a fresh Angel, not the sixth point",
    );
}
