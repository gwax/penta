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

/// Berserk says "destroy", and destruction is what regeneration answers: a
/// Troll with a shield up walks away from the end step it was doomed to,
/// unlike a creature Snuff Out names.
#[test]
fn a_regeneration_shield_saves_the_creature_it_doomed() {
    let (mut game, _lions, held) = staged(1);
    let mut troll = creature(99_500, cards::UTHDEN_TROLL, PlayerId::One);
    troll.attacking = true;
    troll.attacked_this_turn = true;
    let troll_id = troll.card.id;
    game.battlefield.push(troll);

    cast_at(&mut game, held[0], troll_id);
    assert_eq!(power_of(&game, troll_id), Some(4), "a 2/2 doubled");

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    let regenerate = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { source, .. } if *source == troll_id)
        })
        .expect("one red buys a shield");
    game.apply(PlayerId::One, regenerate).expect("it activates");
    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }

    game.step = Step::End;
    game.begin_step_triggers();
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

    let survivor = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == troll_id)
        .expect("the shield answered the destruction");
    assert_eq!(
        survivor.regeneration_shields, 0,
        "and the shield was spent doing it",
    );
    assert!(survivor.tapped, "regenerating taps what it saves");
}

/// Trample is half of what the green mana buys: a doubled Lions blocked by a
/// 1/1 assigns one damage to the blocker and the rest to the player.
#[test]
fn it_grants_trample_and_the_rest_goes_through() {
    let (mut game, lions, held) = staged(1);
    let chump = creature(99_600, cards::SAVANNAH_LIONS, PlayerId::Two);
    let chump_id = chump.card.id;
    game.battlefield.push(chump);
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::DeclareAttackers;
    game.attackers_declared = true;
    game.priority = PlayerId::One;
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == lions)
    {
        permanent.attack_defender = Some(AttackDefender::Player(PlayerId::Two));
    }
    let life = game.players[PlayerId::Two.index()].life;

    cast_at(&mut game, held[0], lions);
    assert_eq!(power_of(&game, lions), Some(4), "a 2/1 doubled");
    assert!(
        game.permanent_has_executable_keyword(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == lions)
                .expect("it is there"),
            KeywordAbility::Trample,
        ),
        "and trampling",
    );

    game.step = Step::DeclareBlockers;
    game.blockers_declared = false;
    game.declare_blocker(chump_id, lions);
    game.finish_declaring_blockers();
    game.deal_combat_damage();
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == chump_id),
        "the blocker took its lethal one",
    );
    assert_eq!(
        game.players[PlayerId::Two.index()].life,
        life - 3,
        "and the other three trampled over it",
    );
}

/// "Destroy that creature *if it attacked this turn*": a Berserk spent on a
/// creature that stayed home is a pump and nothing more.
#[test]
fn a_creature_that_did_not_attack_survives_the_end_step() {
    let (mut game, _lions, held) = staged(1);
    let home = creature(99_700, cards::GRIZZLY_BEARS, PlayerId::One);
    let home_id = home.card.id;
    game.battlefield.push(home);

    cast_at(&mut game, held[0], home_id);
    assert_eq!(power_of(&game, home_id), Some(4), "a 2/2 doubled");

    game.step = Step::End;
    game.begin_step_triggers();
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
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == home_id),
        "it never attacked, so the delayed trigger destroys nothing",
    );
}
