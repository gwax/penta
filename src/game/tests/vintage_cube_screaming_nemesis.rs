//! Screaming Nemesis: damage it takes goes somewhere else, and a player who
//! catches it is out of lifegain for good.

use super::*;

/// The Spirit on the battlefield under Player One.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let nemesis = game
        .put_onto_battlefield(PlayerId::One, cards::SCREAMING_NEMESIS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.players[0].life = 20;
    game.players[1].life = 20;
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, nemesis)
}

/// Answers the trigger, sending its damage at `wanted`.
fn settle(game: &mut Game, wanted: Target) {
    for _ in 0..32 {
        // Two blockers means the attacker divides its damage, which the
        // engine asks about before any of it is dealt.
        if let Some((player, action)) =
            [PlayerId::One, PlayerId::Two]
                .into_iter()
                .find_map(|player| {
                    game.legal_actions(player)
                        .into_iter()
                        .find(|action| matches!(action, Action::AssignCombatDamage { .. }))
                        .map(|action| (player, action))
                })
        {
            game.apply(player, action).expect("the assignment is legal");
            continue;
        }
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options: Vec<_> = decision
                .options
                .iter()
                .filter(|option| match wanted {
                    Target::Player(player) => option.label.contains(match player {
                        PlayerId::One => "you",
                        PlayerId::Two => "opponent",
                    }),
                    Target::Permanent(id) => option.card.is_some_and(|(object, _)| object == id),
                    Target::Card(_) | Target::Spell(_) => false,
                })
                .map(|option| option.id)
                .take(1)
                .collect();
            let options = if options.len() < decision.minimum.max(1) {
                decision
                    .options
                    .iter()
                    .map(|option| option.id)
                    .take(decision.minimum.max(1))
                    .collect()
            } else {
                options
            };
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

/// The damage it takes is passed on, and its size is what it took.
#[test]
fn what_it_takes_it_passes_on() {
    let (mut game, nemesis) = staged();

    game.damage_target_from(None, Some(Target::Permanent(nemesis)), 2);
    settle(&mut game, Target::Player(PlayerId::Two));

    assert_eq!(game.players[1].life, 18, "two dealt to them");
}

/// A player who takes it cannot gain life afterwards, ever.
#[test]
fn a_damaged_player_stops_gaining_life() {
    let (mut game, nemesis) = staged();

    game.damage_target_from(None, Some(Target::Permanent(nemesis)), 1);
    settle(&mut game, Target::Player(PlayerId::Two));
    assert_eq!(game.players[1].life, 19);

    game.gain_life(PlayerId::Two, 5);
    assert_eq!(game.players[1].life, 19, "the gain never arrives");
    game.gain_life(PlayerId::One, 5);
    assert_eq!(
        game.players[0].life, 25,
        "and the other player is untouched by it",
    );
}

/// Sending it at a creature leaves both players able to gain life.
#[test]
fn damaging_a_creature_bars_nobody() {
    let (mut game, nemesis) = staged();
    let bears = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);

    game.damage_target_from(None, Some(Target::Permanent(nemesis)), 2);
    settle(&mut game, Target::Permanent(bears));

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == bears),
        "a 2/2 does not survive two",
    );
    game.gain_life(PlayerId::Two, 3);
    assert_eq!(game.players[1].life, 23, "nobody was told to stop");
}

/// "Any other target": the Spirit is not among its own choices.
#[test]
fn it_cannot_answer_itself() {
    let (mut game, nemesis) = staged();

    game.damage_target_from(None, Some(Target::Permanent(nemesis)), 1);
    for _ in 0..8 {
        if !game.pending_decisions.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }

    let decision = game
        .pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("the trigger asks for a target");
    assert!(
        !decision
            .options
            .iter()
            .any(|option| option.card.is_some_and(|(object, _)| object == nemesis)),
        "any other target leaves it out",
    );
    assert!(
        decision.options.len() >= 2,
        "both players are still on offer",
    );
}

/// Haste is printed on it.
#[test]
fn it_has_haste() {
    let (game, nemesis) = staged();
    let permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == nemesis)
        .expect("it is there");

    assert!(game.permanent_has_executable_keyword(permanent, KeywordAbility::Haste));
    assert_eq!(game.power(permanent), Some(3));
    assert_eq!(game.toughness(permanent), Some(3));
}

/// "If lethal damage is dealt to Screaming Nemesis, its last ability still
/// triggers." Five at a 3/3 kills it, and the scream is still five.
#[test]
fn lethal_damage_still_screams() {
    let (mut game, nemesis) = staged();

    game.damage_target_from(None, Some(Target::Permanent(nemesis)), 5);
    settle(&mut game, Target::Player(PlayerId::Two));

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == nemesis),
        "the Spirit died of it",
    );
    assert_eq!(
        game.players[1].life, 15,
        "and passed on every point on the way out",
    );
}

/// "Once its ability causes it to deal damage to a player, that player won't
/// be able to gain life for the rest of the game. It doesn't matter if
/// Screaming Nemesis remains on the battlefield or not."
#[test]
fn the_lock_outlives_the_spirit() {
    let (mut game, nemesis) = staged();

    game.damage_target_from(None, Some(Target::Permanent(nemesis)), 1);
    settle(&mut game, Target::Player(PlayerId::Two));
    game.move_permanents_to_graveyard(&[nemesis]);
    game.check_state_based_actions();
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == nemesis),
        "the Spirit is gone",
    );

    game.gain_life(PlayerId::Two, 5);
    assert_eq!(
        game.players[1].life, 19,
        "what it did to them is not undone by killing it",
    );
}

/// "If Screaming Nemesis is dealt damage by multiple sources at once, such as
/// by two creatures blocking it, its last ability triggers once and one
/// target is dealt that much damage."
#[test]
fn two_blockers_at_once_are_one_scream() {
    let (mut game, nemesis) = staged();
    let first = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    let second = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.active_player = PlayerId::One;
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.priority = PlayerId::One;

    game.declare_attacker(nemesis, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    settle(&mut game, Target::Player(PlayerId::Two));
    game.step = Step::DeclareBlockers;
    game.blockers_declared = false;
    game.declare_blocker(first, nemesis);
    game.declare_blocker(second, nemesis);
    game.finish_declaring_blockers();
    settle(&mut game, Target::Player(PlayerId::Two));
    for _ in 0..12 {
        if game.step == Step::PostcombatMain {
            break;
        }
        game.advance_step();
        settle(&mut game, Target::Player(PlayerId::Two));
    }

    assert_eq!(
        game.players[1].life, 16,
        "four from two bears, screamed back once rather than twice",
    );
}
