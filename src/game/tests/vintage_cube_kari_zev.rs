//! Kari Zev, Skyship Raider: two mana that attacks as two creatures, one of
//! which goes away again.

use super::*;

/// Kari Zev on the battlefield since last turn, ready to attack.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let kari = game
        .put_onto_battlefield(PlayerId::One, cards::KARI_ZEV_SKYSHIP_RAIDER)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turns_started = [4, 4];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.players[1].life = 20;
    (game, kari)
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
                .take(decision.minimum.max(1))
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

/// Declares Kari Zev as an attacker and lets her trigger resolve.
fn attack(game: &mut Game, kari: GameObjectId) {
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.apply(
        PlayerId::One,
        Action::DeclareAttacker {
            attacker: kari,
            defender: AttackDefender::Player(PlayerId::Two),
        },
    )
    .expect("she may attack");
    game.apply(PlayerId::One, Action::FinishDeclaringAttackers)
        .expect("the declaration is complete");
    settle(game);
}

fn ragavan(game: &Game) -> Option<&Permanent> {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == ObjectKind::Token)
}

/// Her attack brings Ragavan, tapped and attacking beside her.
#[test]
fn attacking_brings_ragavan() {
    let (mut game, kari) = staged();

    attack(&mut game, kari);

    let monkey = ragavan(&game).expect("the Monkey is there");
    assert_eq!(game.power(monkey), Some(2), "a 2/1");
    assert!(monkey.tapped, "tapped");
    assert!(monkey.attacking, "and attacking");
    let _ = kari;
}

/// He is exiled at end of combat rather than left around: the second main
/// phase is Kari Zev alone again.
#[test]
fn ragavan_leaves_at_end_of_combat() {
    let (mut game, kari) = staged();
    attack(&mut game, kari);

    game.step = Step::EndOfCombat;
    game.begin_step_triggers();
    settle(&mut game);

    assert!(ragavan(&game).is_none(), "the Monkey is gone");
    assert_eq!(
        game.players[0].exile.len(),
        0,
        "a token that leaves the battlefield ceases to exist rather than \
         filling exile",
    );
}

/// First strike and menace are the reason she attacks into anything.
#[test]
fn she_strikes_first_and_needs_two_blockers() {
    let (game, kari) = staged();
    let her = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == kari)
        .expect("she is there");

    assert!(game.permanent_has_executable_keyword(her, KeywordAbility::FirstStrike));
    assert!(game.permanent_has_executable_keyword(her, KeywordAbility::Menace));
}

/// The token is Ragavan by name and legendary, which is what stops a second
/// copy of him sticking around.
#[test]
fn the_token_is_the_legendary_ragavan() {
    let (mut game, kari) = staged();

    attack(&mut game, kari);

    let monkey = ragavan(&game).expect("the Monkey is there");
    assert_eq!(
        game.effective_permanent_name(monkey).as_deref(),
        Some("Ragavan"),
    );
    assert!(
        game.effective_rules(monkey)
            .is_some_and(|rules| rules.has_supertype(CardSupertype::Legendary)),
        "and legendary, so a second one would not stay",
    );
}
