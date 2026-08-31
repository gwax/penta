//! Umezawa's Jitte: charge counters, and an activated ability with modes.

use super::*;

/// Equips `source` to `host` by finding the printed equip activation.
fn equip_to(game: &mut Game, source: GameObjectId, host: GameObjectId) {
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
                        .any(|target| *target == Target::Permanent(host))
            }
            _ => false,
        })
        .expect("equip is offered for that creature");
    game.apply(PlayerId::One, action)
        .expect("the ability activates");
    drain_pending(game);
}

/// Activates the Jitte's modal ability, choosing the mode at `index` and
/// whatever target the picked action carries.
fn spend_counter(game: &mut Game, source: GameObjectId, mode: usize, target: Option<GameObjectId>) {
    let wanted = ModeId::from_index(mode).expect("three printed modes");
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source: actual,
                modes,
                targets,
                ..
            } => {
                *actual == source
                    && modes.as_slice() == [wanted]
                    && target.is_none_or(|host| {
                        targets
                            .iter()
                            .flat_map(crate::casting::TargetSelection::targets)
                            .any(|chosen| *chosen == Target::Permanent(host))
                    })
            }
            _ => false,
        })
        .expect("that mode is offered");
    game.apply(PlayerId::One, action)
        .expect("the ability activates");
    drain_pending(game);
}

fn jitte_on_bears(game: &mut Game) -> (GameObjectId, GameObjectId) {
    game.battlefield.clear();
    let jitte = creature(52_000, cards::UMEZAWAS_JITTE, PlayerId::One);
    let jitte_id = jitte.card.id;
    game.battlefield.push(jitte);
    let bears = creature(52_001, cards::GRIZZLY_BEARS, PlayerId::One);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    game.players[PlayerId::One.index()].mana_pool.colorless = 2;
    equip_to(game, jitte_id, bears_id);
    (jitte_id, bears_id)
}

fn permanent(game: &Game, id: GameObjectId) -> &Permanent {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("still on the battlefield")
}

/// Combat damage to a blocking creature charges the Jitte just as a hit to
/// the player would: the printed clause names no recipient.
#[test]
fn combat_damage_to_anything_puts_two_charge_counters_on_the_jitte() {
    let mut game = ready_game();
    let (jitte_id, bears_id) = jitte_on_bears(&mut game);
    let wall = creature(52_002, cards::SERRA_ANGEL, PlayerId::Two);
    let wall_id = wall.card.id;
    game.battlefield.push(wall);

    game.damage_target_from_kind(Some(bears_id), Some(Target::Permanent(wall_id)), 2, true);
    drain_pending(&mut game);

    assert_eq!(
        permanent(&game, jitte_id).counters(CounterKind::named("charge")),
        2,
        "two counters per damage event, not per point",
    );
}

/// The first mode pumps whatever the Jitte equips, and spends one counter.
#[test]
fn the_first_mode_pumps_the_equipped_creature() {
    let mut game = ready_game();
    let (jitte_id, bears_id) = jitte_on_bears(&mut game);
    game.damage_target_from_kind(Some(bears_id), Some(Target::Player(PlayerId::Two)), 2, true);
    drain_pending(&mut game);

    spend_counter(&mut game, jitte_id, 0, None);

    let bears = permanent(&game, bears_id);
    assert_eq!(
        (game.power(bears), game.toughness(bears)),
        (Some(4), Some(4)),
        "a 2/2 with +2/+2",
    );
    assert_eq!(
        permanent(&game, jitte_id).counters(CounterKind::named("charge")),
        1,
        "one of the two counters was spent",
    );
}

/// The second mode shrinks a creature the Jitte never touched, which is what
/// makes it removal. A 1/1 dies outright.
#[test]
fn the_second_mode_shrinks_a_target_creature_to_death() {
    let mut game = ready_game();
    let (jitte_id, bears_id) = jitte_on_bears(&mut game);
    let mouse = creature(52_003, cards::SAVANNAH_LIONS, PlayerId::Two);
    let mouse_id = mouse.card.id;
    game.battlefield.push(mouse);
    game.damage_target_from_kind(Some(bears_id), Some(Target::Player(PlayerId::Two)), 2, true);
    drain_pending(&mut game);

    spend_counter(&mut game, jitte_id, 1, Some(mouse_id));

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != mouse_id),
        "a 2/1 loses its last toughness",
    );
    assert_eq!(
        permanent(&game, jitte_id).counters(CounterKind::named("charge")),
        1
    );
}

/// The third mode touches no permanent at all.
#[test]
fn the_third_mode_gains_two_life() {
    let mut game = ready_game();
    let (jitte_id, bears_id) = jitte_on_bears(&mut game);
    game.damage_target_from_kind(Some(bears_id), Some(Target::Player(PlayerId::Two)), 2, true);
    drain_pending(&mut game);
    let life = game.players[PlayerId::One.index()].life;

    spend_counter(&mut game, jitte_id, 2, None);

    assert_eq!(game.players[PlayerId::One.index()].life, life + 2);
    assert_eq!(
        permanent(&game, jitte_id).counters(CounterKind::named("charge")),
        1
    );
}

/// One printed ability, three offers: the mode is chosen as the ability is
/// activated, so each selection is a legal action of its own. Without a
/// counter to remove, none of them is offered.
#[test]
fn each_mode_is_its_own_activation_and_none_is_offered_without_a_counter() {
    let mut game = ready_game();
    let (jitte_id, bears_id) = jitte_on_bears(&mut game);

    let modal = |game: &Game| {
        game.legal_actions(PlayerId::One)
            .into_iter()
            .filter(|action| {
                matches!(
                    action,
                    Action::ActivateAbility { source, modes, .. }
                        if *source == jitte_id && !modes.is_empty()
                )
            })
            .count()
    };
    assert_eq!(modal(&game), 0, "the cost cannot be paid yet");

    game.damage_target_from_kind(Some(bears_id), Some(Target::Player(PlayerId::Two)), 2, true);
    drain_pending(&mut game);

    // Two modes name nothing, and the third names the only creature on the
    // battlefield -- the Jitte itself is an Equipment.
    assert_eq!(modal(&game), 3);
}

/// "The ability can be used any time its controller has priority -- only the
/// target creature choice has additional requirements. Choosing the +2/+2
/// mode does nothing if the Jitte isn't equipped to a creature when the
/// ability resolves." A bare Jitte still gains life, and still shrinks
/// something, and its pump mode is a counter spent on nothing.
#[test]
fn an_unequipped_jitte_keeps_two_of_its_three_modes() {
    let mut game = ready_game();
    let (jitte_id, bears_id) = jitte_on_bears(&mut game);
    game.damage_target_from_kind(Some(bears_id), Some(Target::Player(PlayerId::Two)), 2, true);
    drain_pending(&mut game);
    assert_eq!(
        permanent(&game, jitte_id).counters(CounterKind::named("charge")),
        2,
        "two counters to spend",
    );

    // The creature wearing it leaves, so the Jitte is bare.
    game.move_permanents_to_graveyard(&[bears_id]);
    game.check_state_based_actions();
    assert!(
        permanent(&game, jitte_id).attached_to.is_none(),
        "an Equipment whose creature is gone is unattached",
    );

    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::SAVANNAH_LIONS)
        .expect("cataloged");
    drain_pending(&mut game);
    let life = game.players[0].life;

    spend_counter(&mut game, jitte_id, 2, None);
    assert_eq!(
        game.players[0].life,
        life + 2,
        "the life mode needs no host"
    );

    spend_counter(&mut game, jitte_id, 1, Some(theirs));
    game.check_state_based_actions();
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == theirs),
        "and neither does the shrink: a 2/1 dies to minus one",
    );
    assert_eq!(
        permanent(&game, jitte_id).counters(CounterKind::named("charge")),
        0,
        "both counters are spent",
    );
}

/// Activates a mode and leaves the ability on the stack, so the board can
/// change underneath it before it resolves.
fn announce(game: &mut Game, source: GameObjectId, mode: usize) {
    let wanted = ModeId::from_index(mode).expect("three printed modes");
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source: actual,
                modes,
                ..
            } => *actual == source && modes.as_slice() == [wanted],
            _ => false,
        })
        .expect("that mode is offered");
    game.apply(PlayerId::One, action)
        .expect("the ability activates");
}

/// "If the Jitte is moved after the +2/+2 mode is announced but before it
/// resolves, the bonus is given to the creature that is equipped when the
/// ability resolves." The mode names no creature as it is announced; it
/// reads the Jitte's host where it resolves.
#[test]
fn the_pump_finds_whatever_is_equipped_when_it_resolves() {
    let mut game = ready_game();
    let (jitte, bears) = jitte_on_bears(&mut game);
    let angel = creature(52_100, cards::SERRA_ANGEL, PlayerId::One);
    let angel_id = angel.card.id;
    game.battlefield.push(angel);
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == jitte)
        .expect("it is there")
        .add_counters(CounterKind::named("charge"), 1);

    announce(&mut game, jitte, 0);
    // Moved while the ability waits, the way any effect that reattaches an
    // Equipment would move it.
    if let Some(equipment) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == jitte)
    {
        equipment.attached_to = Some(angel_id);
    }
    drain_pending(&mut game);

    let host = permanent(&game, angel_id);
    assert_eq!(
        (game.power(host), game.toughness(host)),
        (Some(6), Some(6)),
        "the creature equipped as it resolved is the one that grew",
    );
    let left_behind = permanent(&game, bears);
    assert_eq!(
        (game.power(left_behind), game.toughness(left_behind)),
        (Some(2), Some(2)),
        "and the one it was announced beside is a 2/2 still",
    );
}

/// "If the Jitte leaves the battlefield after the +2/+2 mode is announced
/// but before it resolves, the bonus is given to the creature that was most
/// recently equipped." The counter is spent, the Equipment is gone, and the
/// bonus still lands.
#[test]
fn the_pump_lands_on_the_last_host_even_with_the_jitte_gone() {
    let mut game = ready_game();
    let (jitte, bears) = jitte_on_bears(&mut game);
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == jitte)
        .expect("it is there")
        .add_counters(CounterKind::named("charge"), 1);

    announce(&mut game, jitte, 0);
    game.move_permanents_to_graveyard(&[jitte]);
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != jitte),
        "the Equipment is gone",
    );
    let host = permanent(&game, bears);
    assert_eq!(
        (game.power(host), game.toughness(host)),
        (Some(4), Some(4)),
        "and the creature it was equipping still gets the +2/+2",
    );
}
