//! Ursine Monstrosity: a bear that feeds itself at the beginning of combat.

use super::*;

/// Resolves whatever is on the stack, answering nothing.
fn resolve(game: &mut Game) {
    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
}

/// The Monstrosity on the battlefield, with `library` stacked to be milled
/// and `graveyard` already holding those definitions.
fn staged(library: &[CardDefinitionId], graveyard: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].library.clear();
    game.players[0].graveyard.clear();
    // The top of the library is its last element, so the first milled card
    // is the last one pushed.
    for (offset, definition) in library.iter().enumerate().rev() {
        let id = 92_100 + u32::try_from(offset).expect("a short test library");
        game.players[0]
            .library
            .push(card(id, *definition, PlayerId::One));
    }
    for (offset, definition) in graveyard.iter().enumerate() {
        let id = 92_200 + u32::try_from(offset).expect("a short test graveyard");
        game.players[0]
            .graveyard
            .push(card(id, *definition, PlayerId::One));
    }
    let bear = game
        .put_onto_battlefield(PlayerId::One, cards::URSINE_MONSTROSITY)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    (game, bear)
}

fn begin_combat(game: &mut Game) {
    game.capture_battlefield_triggers(&CommittedTriggerEvent::StepBegins {
        step: TurnStepDef::BeginningOfCombat,
        player: PlayerId::One,
    });
    game.finish_rules_procedure();
    resolve(game);
    drain_pending(game);
}

fn bear_of(game: &Game, id: GameObjectId) -> &Permanent {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("the bear is on the battlefield")
}

/// One card type in the graveyard is +1/+1, counted after the mill that
/// helped put it there.
#[test]
fn the_bonus_counts_card_types_including_what_was_just_milled() {
    let (mut game, bear) = staged(&[cards::GRIZZLY_BEARS], &[]);

    begin_combat(&mut game);

    assert_eq!(game.players[0].graveyard.len(), 1, "a card was milled");
    let bear = bear_of(&game, bear);
    assert_eq!(
        (game.power(bear), game.toughness(bear)),
        (Some(4), Some(4)),
        "a 3/3 plus one card type",
    );
}

/// Card types, not cards: three creatures in the graveyard are still one
/// type, while a creature, an instant, and a land are three.
#[test]
fn the_bonus_counts_types_rather_than_cards() {
    let (mut game, bear) = staged(
        &[cards::GRIZZLY_BEARS],
        &[cards::SAVANNAH_LIONS, cards::SERRA_ANGEL],
    );
    begin_combat(&mut game);
    let one_type = {
        let bear = bear_of(&game, bear);
        (game.power(bear), game.toughness(bear))
    };
    assert_eq!(one_type, (Some(4), Some(4)), "three creatures is one type");

    let (mut game, bear) = staged(
        &[cards::LIGHTNING_BOLT],
        &[cards::GRIZZLY_BEARS, cards::MOUNTAIN],
    );
    begin_combat(&mut game);

    let bear = bear_of(&game, bear);
    assert_eq!(
        (game.power(bear), game.toughness(bear)),
        (Some(6), Some(6)),
        "creature, land, and instant is three types",
    );
}

/// The turn's grant makes it indestructible, so lethal damage leaves it
/// standing.
#[test]
fn the_bear_is_indestructible_for_the_turn() {
    let (mut game, bear) = staged(&[cards::GRIZZLY_BEARS], &[]);
    begin_combat(&mut game);

    game.damage_target_from(None, Some(Target::Permanent(bear)), 20);
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == bear),
        "twenty damage and still there",
    );
}

/// And it has to attack: with the requirement live, finishing the
/// declaration without it is not offered.
#[test]
fn the_bear_must_attack_once_the_trigger_has_resolved() {
    let (mut game, bear) = staged(&[cards::GRIZZLY_BEARS], &[]);
    game.turns_started = [2, 1];
    begin_combat(&mut game);

    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;

    assert!(
        !game
            .legal_actions(PlayerId::One)
            .contains(&Action::FinishDeclaringAttackers),
        "the requirement is live, so the declaration is not finishable yet",
    );

    game.apply(
        PlayerId::One,
        Action::DeclareAttacker {
            attacker: bear,
            defender: AttackDefender::Player(PlayerId::Two),
        },
    )
    .expect("the bear can attack");

    assert!(
        game.legal_actions(PlayerId::One)
            .contains(&Action::FinishDeclaringAttackers),
        "and once it is attacking the requirement is met",
    );
}

/// Without the trigger the bear is an ordinary 3/3 that may stay home: the
/// requirement is granted, not printed.
#[test]
fn the_bear_may_stay_home_before_the_trigger() {
    let (mut game, _bear) = staged(&[cards::GRIZZLY_BEARS], &[]);
    game.turns_started = [2, 1];
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;

    assert!(
        game.legal_actions(PlayerId::One)
            .contains(&Action::FinishDeclaringAttackers),
        "nothing has required it to attack yet",
    );
}

/// "If you can't mill a card, you'll still follow the rest of the
/// instructions": an empty library costs the bear nothing it was promised.
#[test]
fn an_empty_library_still_pays_the_rest() {
    let (mut game, bear) = staged(&[], &[cards::LIGHTNING_BOLT]);
    game.turns_started = [2, 1];

    begin_combat(&mut game);

    assert!(
        game.players[0].library.is_empty(),
        "there was nothing to mill"
    );
    assert_eq!(
        game.power(bear_of(&game, bear)),
        Some(4),
        "the instant already in the graveyard is still one card type",
    );
    assert!(
        game.permanent_has_executable_keyword(bear_of(&game, bear), KeywordAbility::Indestructible),
        "and it is still indestructible",
    );

    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    assert!(
        !game
            .legal_actions(PlayerId::One)
            .contains(&Action::FinishDeclaringAttackers),
        "and it still has to attack",
    );
}

/// "Ursine Monstrosity must attack the chosen player if able, not a
/// planeswalker they control." A declaration that sent it at the
/// planeswalker would satisfy none of the requirement while attacking the
/// player is possible, so it is not offered.
#[test]
fn the_bear_must_attack_the_player_and_not_their_planeswalker() {
    let (mut game, bear) = staged(&[cards::GRIZZLY_BEARS], &[]);
    game.turns_started = [2, 1];
    let walker = game
        .put_onto_battlefield(PlayerId::Two, cards::TEFERI_HERO_OF_DOMINARIA)
        .expect("cataloged");
    drain_pending(&mut game);
    begin_combat(&mut game);
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;

    assert!(
        !game
            .legal_actions(PlayerId::One)
            .contains(&Action::DeclareAttacker {
                attacker: bear,
                defender: AttackDefender::Planeswalker(walker),
            }),
        "the planeswalker is not a legal choice for it",
    );
    assert!(
        game.legal_actions(PlayerId::One)
            .contains(&Action::DeclareAttacker {
                attacker: bear,
                defender: AttackDefender::Player(PlayerId::Two),
            }),
        "the player is what the requirement names",
    );
}

/// The requirement is the bear's own: another creature beside it may still
/// go after the planeswalker.
#[test]
fn the_requirement_does_not_follow_the_rest_of_the_board() {
    let (mut game, _bear) = staged(&[cards::GRIZZLY_BEARS], &[]);
    game.turns_started = [2, 1];
    let walker = game
        .put_onto_battlefield(PlayerId::Two, cards::TEFERI_HERO_OF_DOMINARIA)
        .expect("cataloged");
    let lions = game
        .put_onto_battlefield(PlayerId::One, cards::SAVANNAH_LIONS)
        .expect("cataloged");
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == lions)
    {
        permanent.entered_controller_turn = 0;
    }
    drain_pending(&mut game);
    begin_combat(&mut game);
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;

    assert!(
        game.legal_actions(PlayerId::One)
            .contains(&Action::DeclareAttacker {
                attacker: lions,
                defender: AttackDefender::Planeswalker(walker),
            }),
        "the Lions are under no requirement at all",
    );
}

/// "If, during your declare attackers step, Ursine Monstrosity ... hasn't
/// been under your control continuously since your turn began (and doesn't
/// have haste), then it doesn't attack." The requirement is on it and being
/// unable is what excuses it.
#[test]
fn a_bear_that_arrived_this_turn_stays_home() {
    let (mut game, bear) = staged(&[cards::GRIZZLY_BEARS], &[]);
    game.turns_started = [2, 1];
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == bear)
        .expect("it is there")
        .entered_controller_turn = game.turns_started[PlayerId::One.index()];
    begin_combat(&mut game);

    assert_eq!(
        (
            game.power(bear_of(&game, bear)),
            game.toughness(bear_of(&game, bear))
        ),
        (Some(4), Some(4)),
        "the trigger still resolved and still fed it",
    );

    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;

    assert!(
        !game.legal_actions(PlayerId::One).iter().any(
            |action| matches!(action, Action::DeclareAttacker { attacker, .. } if *attacker == bear)
        ),
        "no haste and no history, so it never had the chance",
    );
    assert!(
        game.legal_actions(PlayerId::One)
            .contains(&Action::FinishDeclaringAttackers),
        "and a bear that cannot attack does not hold the step open",
    );
}

/// "If your turn has multiple combat phases, the last ability triggers at
/// the beginning of each of them." Each one mills its own card and grants
/// its own bonus, worked out as that trigger resolves and kept for the
/// turn: the first is still +1/+1 after the second makes it +2/+2, so a
/// 3/3 that saw two combats is a 6/6 rather than a 5/5.
#[test]
fn a_second_combat_mills_again_and_recounts() {
    let (mut game, bear) = staged(&[cards::GRIZZLY_BEARS, cards::LIGHTNING_BOLT], &[]);
    game.turns_started = [2, 1];

    begin_combat(&mut game);
    assert_eq!(
        game.power(bear_of(&game, bear)),
        Some(4),
        "one creature card in the graveyard is one type",
    );

    begin_combat(&mut game);

    assert_eq!(
        game.players[0].graveyard.len(),
        2,
        "the second combat milled its own card",
    );
    assert_eq!(
        (
            game.power(bear_of(&game, bear)),
            game.toughness(bear_of(&game, bear))
        ),
        (Some(6), Some(6)),
        "one type when the first resolved and two when the second did, both kept",
    );
}
