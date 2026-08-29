//! Berserk: one green mana that reads a creature's power as it resolves and
//! hands back that much again, with the bill due at the end step.

use super::*;

/// An attacking Savannah Lions under Player One, with `berserks` Berserks in
/// hand and the green to cast them.
fn staged(berserks: usize) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let mut lions = creature(99_000, cards::SAVANNAH_LIONS, PlayerId::One);
    lions.attacking = true;
    lions.attacked_this_turn = true;
    let lions_id = lions.card.id;
    game.battlefield.push(lions);
    let mut held = Vec::new();
    for index in 0..berserks {
        let berserk = card(
            99_100 + u32::try_from(index).expect("a few"),
            cards::BERSERK,
            PlayerId::One,
        );
        held.push(berserk.id);
        game.players[0].hand.push(berserk);
    }
    game.add_unrestricted_mana(
        PlayerId::One,
        ManaColor::Green,
        u16::try_from(berserks).expect("a few"),
    );
    (game, lions_id, held)
}

fn cast_at(game: &mut Game, berserk: GameObjectId, target: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == berserk
                    && choices
                        .iter_targets()
                        .copied()
                        .eq(std::iter::once(Target::Permanent(target)))
            }
            _ => false,
        })
        .expect("one green casts it before combat damage");
    game.apply(PlayerId::One, action).expect("it is cast");
    drain_pending(game);
}

fn power_of(game: &Game, id: GameObjectId) -> Option<i16> {
    game.power(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == id)
            .expect("it is on the battlefield"),
    )
}

/// "X is its power" is read as Berserk resolves, so the second one doubles
/// what the first one left behind: a 2/1 becomes a 4/1 and then an 8/1.
#[test]
fn a_second_berserk_doubles_the_doubled_power() {
    let (mut game, lions, held) = staged(2);
    assert_eq!(power_of(&game, lions), Some(2), "a 2/1 to start");

    cast_at(&mut game, held[0], lions);
    assert_eq!(power_of(&game, lions), Some(4), "twice two");

    cast_at(&mut game, held[1], lions);
    assert_eq!(
        power_of(&game, lions),
        Some(8),
        "and twice four: the second one reads what the first one did",
    );
    assert_eq!(
        game.toughness(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == lions)
                .expect("it is there"),
        ),
        Some(1),
        "and neither one touches its toughness"
    );
}

/// The other side of reading X once: the bonus is a fixed number, not a
/// doubling that keeps up. A counter added afterwards is worth exactly one
/// point of power rather than two.
#[test]
fn a_counter_added_afterwards_is_not_doubled() {
    let (mut game, lions, held) = staged(1);
    cast_at(&mut game, held[0], lions);
    assert_eq!(power_of(&game, lions), Some(4));

    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == lions)
        .expect("it is there")
        .add_counters(CounterKind::PlusOnePlusOne, 1);

    assert_eq!(
        power_of(&game, lions),
        Some(5),
        "the +2 Berserk gave is settled: the counter adds its own one",
    );
}
