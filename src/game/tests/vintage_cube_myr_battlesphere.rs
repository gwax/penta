//! Myr Battlesphere: seven mana for eleven power across five bodies, and an
//! attack that cashes the little ones in for damage no blocker can stop.

use super::*;

/// The Battlesphere in hand with the mana for it, and `extra` Myr already
/// out beside it.
fn staged(extra: usize) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    for _ in 0..extra {
        game.put_onto_battlefield(PlayerId::One, cards::MYR_BATTLESPHERE)
            .expect("cataloged");
    }
    let sphere = game
        .build_zone(PlayerId::One, &[cards::MYR_BATTLESPHERE])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = sphere.id;
    game.players[0].hand.push(sphere);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 7);
    game.players[1].life = 20;
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, id)
}

/// Answers whatever is waiting, taking `taken` of whatever is offered.
fn settle(game: &mut Game, taken: usize) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .take(taken.max(decision.minimum))
                .map(|option| option.id)
                .collect();
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the offered choice is legal");
            continue;
        }
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

fn cast(game: &mut Game, sphere: GameObjectId) -> GameObjectId {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == sphere))
        .expect("seven mana casts it");
    game.apply(PlayerId::One, action).expect("it is cast");
    settle(game, 0);
    game.battlefield
        .iter()
        .rev()
        .find(|permanent| permanent.card.definition == cards::MYR_BATTLESPHERE)
        .expect("it resolved")
        .card
        .id
}

fn myr_tokens(game: &Game) -> Vec<&Permanent> {
    game.battlefield
        .iter()
        .filter(|permanent| permanent.card.definition == ObjectKind::Token)
        .collect()
}

fn attack(game: &mut Game, sphere: GameObjectId, taken: usize) {
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.apply(
        PlayerId::One,
        Action::DeclareAttacker {
            attacker: sphere,
            defender: AttackDefender::Player(PlayerId::Two),
        },
    )
    .expect("it attacks");
    game.apply(PlayerId::One, Action::FinishDeclaringAttackers)
        .expect("the declaration finishes");
    settle(game, taken);
}

fn permanent(game: &Game, id: GameObjectId) -> &Permanent {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("it is on the battlefield")
}

/// It arrives with four friends.
#[test]
fn it_makes_four_myr_as_it_enters() {
    let (mut game, sphere) = staged(0);

    cast(&mut game, sphere);

    let made = myr_tokens(&game);
    assert_eq!(made.len(), 4, "four Myr");
    assert_eq!(game.power(made[0]), Some(1));
    assert!(
        game.permanent_types(made[0]).is_some_and(
            |types| types.contains(CardType::Artifact) && types.contains(CardType::Creature)
        ),
        "artifact creatures",
    );
}

/// Tapping all four grows it by four and throws four damage across the
/// table.
#[test]
fn tapping_four_myr_pays_four_damage_and_four_power() {
    let (mut game, sphere) = staged(0);
    let body = cast(&mut game, sphere);
    // The turn it lands it has summoning sickness; give it a turn.
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [6, 5];

    attack(&mut game, body, 4);

    assert_eq!(game.players[1].life, 16, "four damage");
    assert_eq!(
        game.power(permanent(&game, body)),
        Some(8),
        "a 4/7 plus four"
    );
    assert!(
        myr_tokens(&game).iter().all(|token| token.tapped),
        "and every Myr it counted is tapped",
    );
}

/// "You may": tapping none leaves it a plain 4/7 attacker.
#[test]
fn tapping_none_does_nothing() {
    let (mut game, sphere) = staged(0);
    let body = cast(&mut game, sphere);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [6, 5];

    attack(&mut game, body, 0);

    assert_eq!(game.players[1].life, 20, "no damage");
    assert_eq!(game.power(permanent(&game, body)), Some(4), "and no bonus");
    assert!(
        myr_tokens(&game).iter().all(|token| !token.tapped),
        "the Myr are still up",
    );
}

/// Any number: tapping two of the four is two damage.
#[test]
fn the_count_is_whatever_you_tap() {
    let (mut game, sphere) = staged(0);
    let body = cast(&mut game, sphere);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [6, 5];

    attack(&mut game, body, 2);

    assert_eq!(game.players[1].life, 18, "two damage");
    assert_eq!(game.power(permanent(&game, body)), Some(6));
    assert_eq!(
        myr_tokens(&game)
            .iter()
            .filter(|token| token.tapped)
            .count(),
        2,
        "two Myr paid",
    );
}

/// A tapped Myr is not a candidate, so a board of tapped ones pays nothing.
#[test]
fn tapped_myr_cannot_pay() {
    let (mut game, sphere) = staged(0);
    let body = cast(&mut game, sphere);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        if permanent.card.id != body {
            permanent.tapped = true;
        }
    }
    game.turns_started = [6, 5];

    attack(&mut game, body, 4);

    assert_eq!(game.players[1].life, 20, "nothing was there to tap");
    assert_eq!(game.power(permanent(&game, body)), Some(4));
}

/// Attacks `defender` instead of the player across the table.
fn attack_defender(game: &mut Game, sphere: GameObjectId, defender: AttackDefender, taken: usize) {
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.apply(
        PlayerId::One,
        Action::DeclareAttacker {
            attacker: sphere,
            defender,
        },
    )
    .expect("it attacks");
    game.apply(PlayerId::One, Action::FinishDeclaringAttackers)
        .expect("the declaration finishes");
    settle(game, taken);
}

/// "The player or planeswalker it's attacking": a Battlesphere attacking a
/// planeswalker throws its Myr at the planeswalker, not at the player
/// standing behind it.
#[test]
fn the_damage_follows_the_attack_onto_a_planeswalker() {
    let (mut game, sphere) = staged(0);
    let body = cast(&mut game, sphere);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [6, 5];
    let mut walker = creature(10_900, cards::VRASKA_THE_UNSEEN, PlayerId::Two);
    walker.set_counters(CounterKind::Loyalty, 5);
    let walker_id = walker.card.id;
    game.battlefield.push(walker);
    let before = game.players[1].life;

    attack_defender(&mut game, body, AttackDefender::Planeswalker(walker_id), 4);

    assert_eq!(
        game.players[1].life, before,
        "their life is not what the Myr were thrown at",
    );
    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == walker_id)
            .map(|permanent| permanent.counters(CounterKind::Loyalty)),
        Some(1),
        "five loyalty less the four Myr",
    );
}

/// "As the last ability resolves, you can tap untapped Myr you control even
/// if Myr Battlesphere is no longer on the battlefield by then. If that has
/// happened, it won't get the +X/+0, but it will still deal X damage."
#[test]
fn a_dead_sphere_still_throws_its_myr() {
    let (mut game, held) = staged(0);
    let sphere = cast(&mut game, held);
    settle(&mut game, 0);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    assert_eq!(myr_tokens(&game).len(), 4, "four Myr to throw");
    let life = game.players[1].life;

    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.apply(
        PlayerId::One,
        Action::DeclareAttacker {
            attacker: sphere,
            defender: AttackDefender::Player(PlayerId::Two),
        },
    )
    .expect("it attacks");
    game.apply(PlayerId::One, Action::FinishDeclaringAttackers)
        .expect("the declaration finishes");

    // Answered before the trigger resolves: the Myr are still here, and the
    // thing that would have grown is not.
    game.move_permanents_to_graveyard(&[sphere]);
    settle(&mut game, 4);

    assert_eq!(
        game.players[1].life,
        life - 4,
        "the four Myr were thrown all the same",
    );
    assert!(
        myr_tokens(&game).iter().all(|myr| myr.tapped),
        "and they are what paid for it",
    );
}

/// "You can tap any untapped Myr you control, not just the tokens ... this
/// includes Myr that haven't been under your control since your most recent
/// turn began." The ability taps them; it does not ask them to tap
/// themselves, so summoning sickness has nothing to say.
#[test]
fn a_myr_that_just_arrived_can_still_be_tapped() {
    let (mut game, held) = staged(0);
    let sphere = cast(&mut game, held);
    settle(&mut game, 0);
    // Only the Battlesphere is old enough to attack; its Myr arrived with it
    // and stay summoning sick.
    for permanent in &mut game.battlefield {
        if permanent.card.id == sphere {
            permanent.entered_controller_turn = 0;
        }
    }
    let sick = myr_tokens(&game)
        .iter()
        .filter(|myr| myr.entered_controller_turn == game.turns_started[0])
        .count();
    assert_eq!(sick, 4, "all four Myr are new this turn");
    let life = game.players[1].life;

    attack(&mut game, sphere, 4);

    assert_eq!(
        game.players[1].life,
        life - 4,
        "all four paid, however new they are",
    );
    assert!(
        myr_tokens(&game).iter().all(|myr| myr.tapped),
        "and every one of them is tapped for it",
    );
}
