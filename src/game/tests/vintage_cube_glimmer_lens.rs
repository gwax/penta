//! Glimmer Lens: an Equipment that brings its own creature and then asks for
//! a second one before it pays off.

use super::*;

/// The Lens on the battlefield with its Rebel, both player one's.
fn lensed() -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.turns_started[PlayerId::One.index()] = 5;
    let lens = game
        .put_onto_battlefield(PlayerId::One, cards::GLIMMER_LENS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.check_state_based_actions();
    let rebel = game
        .battlefield
        .iter()
        .find(|permanent| {
            is_token_with(
                permanent,
                tokens::creature(&["Rebel"], &[ManaColor::Red], 2, 2),
            )
        })
        .expect("For Mirrodin! made one")
        .card
        .id;
    game.active_player = PlayerId::One;
    game.priority = PlayerId::One;
    (game, lens, rebel)
}

fn attack_with(game: &mut Game, attackers: &[GameObjectId]) {
    game.step = Step::DeclareAttackers;
    for attacker in attackers {
        game.declare_attacker(*attacker, AttackDefender::Player(PlayerId::Two));
    }
    game.finish_declaring_attackers();
    drain_pending(game);
}

fn host_of(game: &Game, equipment: GameObjectId) -> Option<GameObjectId> {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == equipment)
        .and_then(|permanent| permanent.attached_to)
}

fn hand_size(game: &Game) -> usize {
    game.players[PlayerId::One.index()].hand.len()
}

/// For Mirrodin! makes a 2/2 red Rebel and puts the Lens on it.
#[test]
fn it_arrives_with_a_rebel_already_carrying_it() {
    let (game, lens, rebel) = lensed();

    let token = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == rebel)
        .expect("the Rebel is there");
    assert_eq!(
        (game.power(token), game.toughness(token)),
        (Some(2), Some(2))
    );
    assert_eq!(
        host_of(&game, lens),
        Some(rebel),
        "the Equipment attaches itself to what it just made",
    );
}

/// One attacker is not "and at least one other creature".
#[test]
fn the_rebel_attacking_alone_draws_nothing() {
    let (mut game, _, rebel) = lensed();
    let before = hand_size(&game);

    attack_with(&mut game, &[rebel]);

    assert_eq!(hand_size(&game), before);
}

/// Two attackers, one of them the equipped creature: a card.
#[test]
fn the_rebel_plus_a_friend_draws_one() {
    let (mut game, _, rebel) = lensed();
    let friend = creature(70_000, cards::GRIZZLY_BEARS, PlayerId::One);
    let friend_id = friend.card.id;
    game.battlefield.push(friend);
    let before = hand_size(&game);

    attack_with(&mut game, &[rebel, friend_id]);

    assert_eq!(hand_size(&game), before + 1);
}

/// The trigger is the equipped creature's, not any two attackers'. A pair
/// that leaves the Rebel at home draws nothing.
#[test]
fn two_other_attackers_without_the_equipped_one_draw_nothing() {
    let (mut game, _, _) = lensed();
    let first = creature(70_100, cards::GRIZZLY_BEARS, PlayerId::One);
    let first_id = first.card.id;
    game.battlefield.push(first);
    let second = creature(70_101, cards::SAVANNAH_LIONS, PlayerId::One);
    let second_id = second.card.id;
    game.battlefield.push(second);
    let before = hand_size(&game);

    attack_with(&mut game, &[first_id, second_id]);

    assert_eq!(hand_size(&game), before);
}

/// Equip moves the trigger's subject. Once the Lens is on the Bears, the
/// Bears and one other attacker are what draws.
#[test]
fn equipping_something_else_moves_the_trigger() {
    let (mut game, lens, rebel) = lensed();
    let bears = creature(70_200, cards::GRIZZLY_BEARS, PlayerId::One);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    game.players[PlayerId::One.index()].mana_pool.colorless = 1;
    game.players[PlayerId::One.index()].mana_pool.white = 1;
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } => {
                *source == lens
                    && targets
                        .iter()
                        .flat_map(crate::casting::TargetSelection::targets)
                        .any(|target| *target == Target::Permanent(bears_id))
            }
            _ => false,
        })
        .expect("equip is offered for the Bears");
    game.apply(PlayerId::One, action).expect("it activates");
    drain_pending(&mut game);
    assert_eq!(host_of(&game, lens), Some(bears_id));

    let before = hand_size(&game);
    attack_with(&mut game, &[bears_id, rebel]);
    assert_eq!(hand_size(&game), before + 1);
}

/// "If the Rebel is destroyed, the Equipment stays on the battlefield." What
/// it loses is its subject: with nothing equipped there is no "equipped
/// creature" to attack, so a full attack draws nothing -- and the equip cost
/// is still there to give it a new host.
#[test]
fn the_rebel_dying_leaves_the_lens_behind() {
    let (mut game, lens, rebel) = lensed();
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == rebel)
        .expect("the Rebel is there")
        .damage = 2;
    game.check_state_based_actions();
    drain_pending(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == rebel),
        "two damage is lethal to a 2/2",
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == lens),
        "the Equipment is not attached to what it lost",
    );
    assert_eq!(host_of(&game, lens), None, "and it equips nothing");

    let first = creature(70_300, cards::GRIZZLY_BEARS, PlayerId::One);
    let first_id = first.card.id;
    game.battlefield.push(first);
    let second = creature(70_301, cards::SAVANNAH_LIONS, PlayerId::One);
    let second_id = second.card.id;
    game.battlefield.push(second);
    let before = hand_size(&game);
    attack_with(&mut game, &[first_id, second_id]);
    assert_eq!(
        hand_size(&game),
        before,
        "an unequipped Lens has no attacker of its own to count",
    );

    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.players[PlayerId::One.index()].mana_pool.colorless = 1;
    game.players[PlayerId::One.index()].mana_pool.white = 1;
    assert!(
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|action| match action {
                Action::ActivateAbility {
                    source, targets, ..
                } => {
                    *source == lens
                        && targets
                            .iter()
                            .flat_map(crate::casting::TargetSelection::targets)
                            .any(|target| *target == Target::Permanent(first_id))
                }
                _ => false,
            }),
        "the equip cost still moves it onto something else",
    );
}

/// "The Rebel enters the battlefield as a 2/2 creature, then the Equipment
/// becomes attached to it. Abilities that trigger when a creature enters the
/// battlefield see that a 2/2 creature entered." The Goliath counts the
/// power of what arrived, and what arrived was a 2/2.
#[test]
fn what_watches_creatures_enter_sees_a_two_two() {
    let mut game = ready_game();
    game.battlefield.clear();
    let goliath = game
        .put_onto_battlefield(PlayerId::One, cards::HAMLETBACK_GOLIATH)
        .expect("cataloged");
    drain_pending(&mut game);
    let base = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == goliath)
        .map(|permanent| game.power(permanent))
        .expect("the Goliath is there");
    assert_eq!(base, Some(6));

    game.put_onto_battlefield(PlayerId::One, cards::GLIMMER_LENS)
        .expect("cataloged");
    for _ in 0..12 {
        if !game.pending_decisions.is_empty() {
            break;
        }
        let player = game.priority;
        game.apply(player, Action::PassPriority)
            .expect("the trigger goes on the stack and resolves");
    }
    choose_decision_by_label(&mut game, PlayerId::One, "Do it");
    drain_pending(&mut game);

    let grown = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == goliath)
        .map(|permanent| game.power(permanent))
        .expect("the Goliath is still there");
    assert_eq!(
        grown,
        Some(8),
        "two counters for the 2/2 that entered, not zero for a token yet to be sized",
    );
}

/// "At least one other creature" is a condition rather than a count: three
/// attackers with the Lens among them is still one card.
#[test]
fn a_wider_attack_still_draws_one() {
    let (mut game, _lens, rebel) = lensed();
    let mut friends = Vec::new();
    for index in 0..2 {
        let bears = creature(70_400 + index, cards::GRIZZLY_BEARS, PlayerId::One);
        friends.push(bears.card.id);
        game.battlefield.push(bears);
    }
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    let before = hand_size(&game);

    attack_with(&mut game, &[rebel, friends[0], friends[1]]);

    assert_eq!(
        hand_size(&game),
        before + 1,
        "one trigger, however many came with it",
    );
}

/// Equip is sorcery speed: the Lens cannot be moved onto whichever creature
/// the attack wants once the attack is under way.
#[test]
fn the_lens_cannot_be_moved_mid_combat() {
    let (mut game, lens, rebel) = lensed();
    let bears = creature(70_500, cards::GRIZZLY_BEARS, PlayerId::One);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.players[PlayerId::One.index()].mana_pool.colorless = 1;
    game.players[PlayerId::One.index()].mana_pool.white = 1;

    let equip_offered = |game: &Game| {
        game.legal_actions(PlayerId::One).iter().any(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == lens),
        )
    };

    attack_with(&mut game, &[bears_id, rebel]);
    game.priority = PlayerId::One;
    assert!(!equip_offered(&game), "not with attackers already declared");

    game.step = Step::PostcombatMain;
    assert!(
        equip_offered(&game),
        "and the main phase after it is where the equip cost may be paid",
    );
    assert_eq!(
        host_of(&game, lens),
        Some(rebel),
        "so it spent the combat on the creature it started on",
    );
}
