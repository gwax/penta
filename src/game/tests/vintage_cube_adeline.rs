//! Adeline, Resplendent Cathar: every attack makes a body, and every body
//! makes her bigger before damage.

use super::*;

/// Player One with Adeline out since last turn, plus `extra` beside her,
/// ready to declare attackers.
fn staged(extra: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let adeline = game
        .put_onto_battlefield(PlayerId::One, cards::ADELINE_RESPLENDENT_CATHAR)
        .expect("cataloged");
    for definition in extra {
        game.put_onto_battlefield(PlayerId::One, *definition)
            .expect("cataloged");
    }
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [2, 1];
    drain_pending(&mut game);
    game.active_player = PlayerId::One;
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.priority = PlayerId::One;
    (game, adeline)
}

fn settle(game: &mut Game) {
    for _ in 0..24 {
        if !game.pending_decisions.is_empty() {
            return;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
    game.check_state_based_actions();
}

fn attack_with(game: &mut Game, attackers: &[GameObjectId]) {
    for attacker in attackers {
        game.apply(
            PlayerId::One,
            Action::DeclareAttacker {
                attacker: *attacker,
                defender: AttackDefender::Player(PlayerId::Two),
            },
        )
        .expect("it attacks");
    }
    game.apply(PlayerId::One, Action::FinishDeclaringAttackers)
        .expect("the declaration finishes");
    settle(game);
}

fn permanent(game: &Game, id: GameObjectId) -> &Permanent {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("it is on the battlefield")
}

fn humans(game: &Game) -> Vec<&Permanent> {
    game.battlefield
        .iter()
        .filter(|permanent| {
            is_token_with(
                permanent,
                tokens::creature(&["Human"], &[ManaColor::White], 1, 1),
            )
        })
        .collect()
}

/// Alone on an empty board she is a 1/4: she counts herself.
#[test]
fn her_power_counts_herself() {
    let (game, adeline) = staged(&[]);

    assert_eq!(game.power(permanent(&game, adeline)), Some(1));
    assert_eq!(
        game.toughness(permanent(&game, adeline)),
        Some(4),
        "the printed toughness is left alone",
    );
}

/// Every other creature you control adds one, and only yours do.
#[test]
fn her_power_counts_every_creature_you_control() {
    let (mut game, adeline) = staged(&[cards::SAVANNAH_LIONS, cards::GRIZZLY_BEARS]);
    assert_eq!(
        game.power(permanent(&game, adeline)),
        Some(3),
        "three of them"
    );

    game.put_onto_battlefield(PlayerId::Two, cards::SERRA_ANGEL)
        .expect("cataloged");
    drain_pending(&mut game);

    assert_eq!(
        game.power(permanent(&game, adeline)),
        Some(3),
        "and their Angel is not one of yours",
    );
}

/// Attacking makes a Human, already tapped and already attacking.
#[test]
fn attacking_makes_a_tapped_attacking_human() {
    let (mut game, adeline) = staged(&[]);

    attack_with(&mut game, &[adeline]);

    let humans = humans(&game);
    assert_eq!(humans.len(), 1, "one opponent, one token");
    let token = humans[0];
    assert!(token.tapped, "tapped");
    assert!(token.attacking, "and attacking");
    assert_eq!(
        token.attack_defender,
        Some(AttackDefender::Player(PlayerId::Two)),
        "at the one player there is to attack",
    );
    assert_eq!(
        (game.power(token), game.toughness(token)),
        (Some(1), Some(1))
    );
}

/// The token is a creature you control, so it makes her bigger -- and it
/// arrives before combat damage, which is the whole point.
#[test]
fn the_token_makes_her_bigger_before_damage() {
    let (mut game, adeline) = staged(&[]);
    assert_eq!(
        game.power(permanent(&game, adeline)),
        Some(1),
        "a 1/4 so far"
    );

    attack_with(&mut game, &[adeline]);

    assert_eq!(
        game.power(permanent(&game, adeline)),
        Some(2),
        "herself and the Human she just made",
    );
}

/// "Whenever you attack" is one declaration, not one attacker: attacking
/// with three creatures still makes one token.
#[test]
fn a_wide_attack_still_makes_one_token() {
    let (mut game, adeline) = staged(&[cards::SAVANNAH_LIONS, cards::GRIZZLY_BEARS]);
    let others = game
        .battlefield
        .iter()
        .filter(|permanent| permanent.card.definition != cards::ADELINE_RESPLENDENT_CATHAR)
        .map(|permanent| permanent.card.id)
        .collect::<Vec<_>>();

    attack_with(&mut game, &[adeline, others[0], others[1]]);

    assert_eq!(humans(&game).len(), 1, "one declaration, one token");
}

/// She need not be one of the attackers: "whenever you attack" watches the
/// declaration, not her.
#[test]
fn she_triggers_when_something_else_attacks() {
    let (mut game, _adeline) = staged(&[cards::SAVANNAH_LIONS]);
    let lions = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::SAVANNAH_LIONS)
        .expect("it is here")
        .card
        .id;

    attack_with(&mut game, &[lions]);

    assert_eq!(
        humans(&game).len(),
        1,
        "the Lions attacking is you attacking"
    );
}

/// Vigilance: she attacks without tapping.
#[test]
fn she_attacks_untapped() {
    let (mut game, adeline) = staged(&[]);
    assert!(
        game.permanent_has_executable_keyword(permanent(&game, adeline), KeywordAbility::Vigilance)
    );

    attack_with(&mut game, &[adeline]);

    assert!(
        !permanent(&game, adeline).tapped,
        "vigilance kept her untapped",
    );
}

/// A characteristic-defining ability functions in every zone (CR 604.3).
/// From a graveyard she reads the same board -- minus herself, since she is
/// no longer one of the creatures you control.
#[test]
fn her_power_answers_from_a_graveyard() {
    let (mut game, adeline) = staged(&[cards::SAVANNAH_LIONS, cards::GRIZZLY_BEARS]);
    assert_eq!(game.power(permanent(&game, adeline)), Some(3));

    game.move_target_to_zone(
        Target::Permanent(adeline),
        ZoneKind::Graveyard,
        ZoneMoveCause::Effect {
            controller: PlayerId::One,
        },
        None,
        ZonePlacement::Top,
    );
    drain_pending(&mut game);
    let card = game.players[0]
        .graveyard
        .last()
        .expect("she is in the graveyard")
        .id;

    assert_eq!(
        game.current_or_last_known_power(card),
        Some(2),
        "the two creatures still out, and no longer herself",
    );
    assert_eq!(
        game.current_or_last_known_toughness(card),
        Some(4),
        "the printed toughness is what she has anywhere",
    );
}

/// Reading her in a hand asks the same question of the same board: nothing
/// about the zone changes what the count counts.
#[test]
fn her_power_answers_from_a_hand() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let card = game
        .build_zone(PlayerId::One, &[cards::ADELINE_RESPLENDENT_CATHAR])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let card_id = card.id;
    game.players[0].hand.push(card);

    assert_eq!(
        game.current_or_last_known_power(card_id),
        Some(0),
        "an empty board is no creatures at all",
    );

    game.put_onto_battlefield(PlayerId::One, cards::SAVANNAH_LIONS)
        .expect("cataloged");
    drain_pending(&mut game);

    assert_eq!(
        game.current_or_last_known_power(card_id),
        Some(1),
        "and one creature is one power, while she waits in hand",
    );
}
