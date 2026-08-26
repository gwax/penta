//! Questing Beast: four abilities that each answer one of the ways a four-
//! mana attacker is usually stopped.

use super::*;

/// The Beast on the battlefield since last turn, in Player One's
/// declare-attackers step.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[1].hand.clear();
    let beast = game
        .put_onto_battlefield(PlayerId::One, cards::QUESTING_BEAST)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.players[0].life = 20;
    game.players[1].life = 20;
    game.turns_started = [2, 1];
    game.active_player = PlayerId::One;
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.priority = PlayerId::One;
    (game, beast)
}

fn settle(game: &mut Game) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .map(|option| option.id)
                .take(decision.minimum.max(1).min(decision.maximum))
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

fn permanent(game: &Game, id: GameObjectId) -> &Permanent {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("it is on the battlefield")
}

/// Attacks with `attackers` and runs combat damage.
fn attack(game: &mut Game, attackers: &[GameObjectId]) {
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    for attacker in attackers {
        game.declare_attacker(*attacker, AttackDefender::Player(PlayerId::Two));
    }
    game.finish_declaring_attackers();
    settle(game);
    game.step = Step::CombatDamage;
    game.deal_combat_damage();
    settle(game);
}

/// The three printed keywords.
#[test]
fn it_has_vigilance_deathtouch_and_haste() {
    let (game, beast) = staged();
    let body = permanent(&game, beast);

    for keyword in [
        KeywordAbility::Vigilance,
        KeywordAbility::Deathtouch,
        KeywordAbility::Haste,
    ] {
        assert!(
            game.permanent_has_executable_keyword(body, keyword),
            "{keyword:?}",
        );
    }
    assert_eq!(game.power(body), Some(4));
    assert_eq!(game.toughness(body), Some(4));
}

/// Haste in practice: it attacks the turn it lands.
#[test]
fn it_attacks_the_turn_it_arrives() {
    let (mut game, beast) = staged();
    let arrived = game.turn;
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = arrived;
        permanent.entered_turn = arrived;
    }

    attack(&mut game, &[beast]);

    assert_eq!(
        game.players[1].life, 16,
        "four damage on the turn it landed"
    );
}

/// A 2/2 cannot block it, and a 3/3 can.
#[test]
fn small_creatures_cannot_block_it() {
    let (mut game, beast) = staged();
    let bears = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    let titan = game
        .put_onto_battlefield(PlayerId::Two, cards::GRAVE_TITAN)
        .expect("cataloged");
    drain_pending(&mut game);
    game.declare_attacker(beast, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    settle(&mut game);
    game.step = Step::DeclareBlockers;

    let offered = |game: &Game, blocker: GameObjectId| {
        game.legal_actions(PlayerId::Two).into_iter().any(|action| {
            matches!(
                action,
                Action::DeclareBlocker { blocker: actual, attacker }
                    if actual == blocker && attacker == beast
            )
        })
    };
    assert!(
        !offered(&game, bears),
        "a 2/2 is power 2 or less, so blocking is not on offer",
    );
    assert!(offered(&game, titan), "a 6/6 blocks it the ordinary way");
}

/// A Fog stops an ordinary attacker's combat damage.
#[test]
fn a_fog_stops_an_ordinary_attacker() {
    let (mut game, beast) = staged();
    game.battlefield
        .retain(|permanent| permanent.card.id != beast);
    let bears = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    attack_through_a_fog(&mut game, &[bears]);

    assert_eq!(game.players[1].life, 20, "the Fog did its work");
}

/// With the Beast out, the same Fog stops nothing -- not the Beast's own
/// damage, and not the bear's either.
#[test]
fn a_fog_stops_none_of_your_creatures_with_the_beast_out() {
    let (mut game, beast) = staged();
    let bears = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    attack_through_a_fog(&mut game, &[beast, bears]);

    assert_eq!(
        game.players[1].life, 14,
        "four from the Beast and two from the bear, none of it prevented",
    );
}

/// Attacks with `attackers`, lets Player Two cast a Fog once the attack is
/// declared, and only then runs combat damage.
fn attack_through_a_fog(game: &mut Game, attackers: &[GameObjectId]) {
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    for attacker in attackers {
        game.declare_attacker(*attacker, AttackDefender::Player(PlayerId::Two));
    }
    game.finish_declaring_attackers();
    settle(game);

    let card = game
        .build_zone(PlayerId::Two, &[cards::FOG])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let held = card.id;
    game.players[1].hand.push(card);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Green, 1);
    game.step = Step::DeclareBlockers;
    game.priority = PlayerId::Two;
    game.apply(PlayerId::Two, Action::FinishDeclaringBlockers)
        .expect("nothing blocks");
    game.priority = PlayerId::Two;
    let cast = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == held))
        .expect("they can cast it");
    game.apply(PlayerId::Two, cast).expect("it is cast");
    settle(game);

    game.step = Step::CombatDamage;
    game.deal_combat_damage();
    settle(game);
}

/// Connecting with a player takes a planeswalker that player controls down
/// by the same amount.
#[test]
fn it_hits_a_planeswalker_for_the_same_damage() {
    let (mut game, beast) = staged();
    let teferi = game
        .put_onto_battlefield(PlayerId::Two, cards::TEFERI_HERO_OF_DOMINARIA)
        .expect("cataloged");
    drain_pending(&mut game);

    attack(&mut game, &[beast]);

    assert_eq!(game.players[1].life, 16, "the player took four");
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == teferi),
        "and four loyalty is all Teferi had",
    );
}

/// No planeswalker, no trigger to speak of: the damage to the player is the
/// whole of it.
#[test]
fn it_needs_a_planeswalker_to_point_at() {
    let (mut game, beast) = staged();

    attack(&mut game, &[beast]);

    assert_eq!(game.players[1].life, 16);
}
