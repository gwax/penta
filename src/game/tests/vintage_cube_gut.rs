//! Gut, True Soul Zealot: a spare permanent turned into four attacking
//! power that two blockers cannot answer alone.

use super::*;

fn settle(game: &mut Game, sacrifice: bool) {
    for _ in 0..16 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let wanted = if sacrifice {
                decision.options.first()
            } else {
                None
            };
            let options = wanted.map(|option| vec![option.id]).unwrap_or_default();
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the decision accepts what it offered");
            continue;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
}

/// Gut on the battlefield alongside `others`, ready to attack.
fn staged(others: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    let gut = game
        .put_onto_battlefield(PlayerId::One, cards::GUT_TRUE_SOUL_ZEALOT)
        .expect("cataloged");
    for definition in others {
        game.put_onto_battlefield(PlayerId::One, *definition)
            .expect("cataloged");
    }
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [2, 1];
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    (game, gut)
}

fn attack_with(game: &mut Game, attacker: GameObjectId, sacrifice: bool) {
    game.apply(
        PlayerId::One,
        Action::DeclareAttacker {
            attacker,
            defender: AttackDefender::Player(PlayerId::Two),
        },
    )
    .expect("it attacks");
    game.apply(PlayerId::One, Action::FinishDeclaringAttackers)
        .expect("the declaration finishes");
    settle(game, sacrifice);
    drain_pending(game);
}

fn skeleton(game: &Game) -> Option<&Permanent> {
    game.battlefield.iter().find(|permanent| {
        is_token_with(
            permanent,
            token_with_menace(tokens::creature(&["Skeleton"], &[ManaColor::Black], 4, 1)),
        )
    })
}

/// Attacking with something to spend makes a Skeleton already attacking.
#[test]
fn the_skeleton_arrives_tapped_and_attacking() {
    let (mut game, gut) = staged(&[cards::BLACK_LOTUS]);

    attack_with(&mut game, gut, true);

    let skeleton = skeleton(&game).expect("the Skeleton was made");
    assert!(skeleton.tapped, "tapped");
    assert!(skeleton.attacking, "and attacking");
    assert_eq!(
        skeleton.attack_defender,
        Some(AttackDefender::Player(PlayerId::Two)),
        "at the one player there is to attack",
    );
    assert_eq!(
        (game.power(skeleton), game.toughness(skeleton)),
        (Some(4), Some(1)),
    );
    assert!(
        game.permanent_has_executable_keyword(skeleton, KeywordAbility::Menace),
        "with menace",
    );
    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.definition != cards::BLACK_LOTUS),
        "and the Lotus paid for it",
    );
}

/// Declining the sacrifice makes nothing.
#[test]
fn declining_the_sacrifice_makes_no_skeleton() {
    let (mut game, gut) = staged(&[cards::BLACK_LOTUS]);

    attack_with(&mut game, gut, false);

    assert!(skeleton(&game).is_none(), "nothing was spent");
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::BLACK_LOTUS),
        "and the Lotus is still there",
    );
}

/// Gut cannot eat itself: the sacrifice names another permanent.
#[test]
fn gut_is_not_a_legal_sacrifice() {
    let (mut game, gut) = staged(&[]);
    game.apply(
        PlayerId::One,
        Action::DeclareAttacker {
            attacker: gut,
            defender: AttackDefender::Player(PlayerId::Two),
        },
    )
    .expect("it attacks");
    game.apply(PlayerId::One, Action::FinishDeclaringAttackers)
        .expect("the declaration finishes");
    drain_pending(&mut game);
    while !game.stack.is_empty() {
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == gut),
        "Gut is still there",
    );
    assert!(skeleton(&game).is_none(), "and made nothing");
}

/// "Whenever you attack" is the declaration, not each attacker: attacking
/// with Gut and two others still offers the sacrifice once.
#[test]
fn a_wide_attack_triggers_it_once() {
    let (mut game, gut) = staged(&[cards::BLACK_LOTUS, cards::SAVANNAH_LIONS]);
    let lions = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::SAVANNAH_LIONS)
        .expect("it is here")
        .card
        .id;
    for attacker in [gut, lions] {
        game.apply(
            PlayerId::One,
            Action::DeclareAttacker {
                attacker,
                defender: AttackDefender::Player(PlayerId::Two),
            },
        )
        .expect("it attacks");
    }
    game.apply(PlayerId::One, Action::FinishDeclaringAttackers)
        .expect("the declaration finishes");
    settle(&mut game, true);
    drain_pending(&mut game);

    assert_eq!(
        game.battlefield
            .iter()
            .filter(|permanent| is_token_with(
                permanent,
                token_with_menace(tokens::creature(&["Skeleton"], &[ManaColor::Black], 4, 1))
            ))
            .count(),
        1,
        "two attackers, one trigger, one Skeleton",
    );
}

/// "Gut doesn't have to be among the attacking creatures." The trigger
/// watches the declaration rather than her.
#[test]
fn she_need_not_attack_herself() {
    let (mut game, gut) = staged(&[cards::GRIZZLY_BEARS, cards::BLACK_LOTUS]);
    let bears = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::GRIZZLY_BEARS)
        .expect("staged")
        .card
        .id;

    attack_with(&mut game, bears, true);

    assert!(
        skeleton(&game).is_some(),
        "somebody attacked, which is all the trigger asks",
    );
    assert!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == gut)
            .is_some_and(|permanent| !permanent.attacking),
        "and Gut stayed home while it happened",
    );
}

/// Answers what is asked, taking the first option that is not a refusal:
/// the sacrifice, and any draw offered along the way.
fn settle_taking_offers(game: &mut Game) {
    for _ in 0..16 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .find(|option| option.label != "Decline")
                .map(|option| vec![option.id])
                .unwrap_or_default();
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the decision accepts what it offered");
            continue;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
}

/// "Although the Skeleton is an attacking creature, it was never declared as
/// an attacking creature ... abilities that trigger whenever a creature
/// attacks won't trigger when it enters the battlefield attacking." An
/// Isperia across the table sees the one attacker that was declared.
#[test]
fn the_skeleton_is_attacking_without_having_attacked() {
    let (mut game, gut) = staged(&[cards::BLACK_LOTUS]);
    game.put_onto_battlefield(PlayerId::Two, cards::ISPERIA_SUPREME_JUDGE)
        .expect("cataloged");
    drain_pending(&mut game);
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    let held = game.players[1].hand.len();

    game.apply(
        PlayerId::One,
        Action::DeclareAttacker {
            attacker: gut,
            defender: AttackDefender::Player(PlayerId::Two),
        },
    )
    .expect("she attacks");
    game.apply(PlayerId::One, Action::FinishDeclaringAttackers)
        .expect("the declaration finishes");
    settle_taking_offers(&mut game);

    let skeleton = skeleton(&game).expect("the Skeleton was made");
    assert!(skeleton.attacking, "it is attacking");
    assert_eq!(
        game.players[1].hand.len(),
        held + 1,
        "and the one card is for Gut alone: the Skeleton never declared",
    );
}
