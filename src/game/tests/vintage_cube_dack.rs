//! Dack Fayden: rooting through a library, stealing an artifact, and an
//! emblem that takes whatever your spells point at.

use super::*;

fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    let dack = game
        .put_onto_battlefield(PlayerId::One, cards::DACK_FAYDEN)
        .expect("cataloged");
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == dack)
    {
        permanent.entered_controller_turn = 0;
    }
    drain_pending(&mut game);
    (game, dack)
}

fn activate(game: &mut Game, dack: GameObjectId, index: usize, targets: Vec<TargetSelection>) {
    let ability = activated_ability_for(game, dack, index);
    game.apply(
        PlayerId::One,
        Action::ActivateAbility {
            source: dack,
            ability,
            targets,
            cost_objects: Vec::new(),
            x: 0,
            modes: Vec::new(),
            mana_payment: None,
        },
    )
    .expect("the loyalty ability activates");
}

/// Answers every pending decision with its first option, then resolves
/// whatever is left on the stack.
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
                .take(decision.minimum.max(1))
                .map(|option| option.id)
                .collect();
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
            return;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            return;
        }
    }
}

fn controller_of(game: &Game, permanent: GameObjectId) -> Option<PlayerId> {
    game.battlefield
        .iter()
        .find(|candidate| candidate.card.id == permanent)
        .map(|candidate| candidate.controller)
}

/// Two in and two out, for whoever is pointed at.
#[test]
fn the_plus_one_digs_two_and_pitches_two() {
    let (mut game, dack) = staged();
    let before = game.players[1].hand.len();

    activate(
        &mut game,
        dack,
        0,
        vec![TargetSelection::single(
            TargetSlotId(0),
            Target::Player(PlayerId::Two),
        )],
    );
    settle(&mut game);

    assert_eq!(
        game.players[1].hand.len(),
        before,
        "two drawn and two discarded is a wash in hand size",
    );
    assert_eq!(game.players[1].graveyard.len(), 2, "and two in the bin");
}

/// The artifact changes hands and stays there.
#[test]
fn the_minus_two_steals_an_artifact() {
    let (mut game, dack) = staged();
    let mox = game
        .put_onto_battlefield(PlayerId::Two, cards::MOX_SAPPHIRE)
        .expect("cataloged");
    drain_pending(&mut game);
    assert_eq!(controller_of(&game, mox), Some(PlayerId::Two));

    activate(
        &mut game,
        dack,
        1,
        vec![TargetSelection::single(
            TargetSlotId(0),
            Target::Permanent(mox),
        )],
    );
    settle(&mut game);

    assert_eq!(controller_of(&game, mox), Some(PlayerId::One));
}

/// The emblem reads the targets a spell has already chosen, so the creature
/// is stolen before the bolt that pointed at it ever resolves.
#[test]
fn the_emblem_steals_what_a_spell_points_at() {
    let (mut game, dack) = staged();
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == dack)
    {
        permanent.set_counters(CounterKind::Loyalty, 6);
    }
    let bears = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);

    activate(&mut game, dack, 2, Vec::new());
    settle(&mut game);
    assert_eq!(game.emblems.len(), 1, "the emblem is in the command zone");

    let bolt = game
        .build_zone(PlayerId::One, &[cards::LIGHTNING_BOLT])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    game.players[0].hand.push(bolt);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { choices, .. } => choices
                .iter_targets()
                .any(|target| *target == Target::Permanent(bears)),
            _ => false,
        })
        .expect("the bolt can point at the bears");
    game.apply(PlayerId::One, cast)
        .expect("the bolt is castable");
    assert_eq!(game.stack.len(), 2, "the trigger went on top of the bolt");

    // Resolve only the trigger, which went on the stack above the bolt.
    while game.stack.len() > 1 || !game.pending_triggers.is_empty() {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options: vec![decision.options[0].id],
                },
            )
            .expect("the decision accepts what it offered");
            continue;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
    assert_eq!(
        controller_of(&game, bears),
        Some(PlayerId::One),
        "the theft happens while the spell is still on the stack",
    );

    settle(&mut game);
    assert_eq!(
        controller_of(&game, bears),
        None,
        "the bolt still killed it -- the theft does not save it",
    );
    assert!(
        game.players[1]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS),
        "and it goes to its owner's graveyard, not the thief's",
    );
}

/// A spell that points at nobody's permanent leaves the emblem quiet.
#[test]
fn the_emblem_ignores_a_spell_that_targets_a_player() {
    let (mut game, dack) = staged();
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == dack)
    {
        permanent.set_counters(CounterKind::Loyalty, 6);
    }
    activate(&mut game, dack, 2, Vec::new());
    settle(&mut game);

    let bolt = game
        .build_zone(PlayerId::One, &[cards::LIGHTNING_BOLT])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    game.players[0].hand.push(bolt);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { choices, .. } => choices
                .iter_targets()
                .any(|target| *target == Target::Player(PlayerId::Two)),
            _ => false,
        })
        .expect("the bolt can point at the opponent");
    game.apply(PlayerId::One, cast)
        .expect("the bolt is castable");

    assert!(
        game.pending_triggers.is_empty() && game.stack.len() == 1,
        "no permanent was targeted, so nothing triggered",
    );
}

/// "The effect of Dack's second ability ... lasts indefinitely. You won't
/// lose control of the permanents if Dack leaves the battlefield." A control
/// change with no stated duration is not held by its source, so answering
/// Dack afterwards does not buy the Mox back.
#[test]
fn the_stolen_artifact_stays_when_dack_dies() {
    let (mut game, dack) = staged();
    let mox = game
        .put_onto_battlefield(PlayerId::Two, cards::MOX_SAPPHIRE)
        .expect("cataloged");
    drain_pending(&mut game);

    activate(
        &mut game,
        dack,
        1,
        vec![TargetSelection::single(
            TargetSlotId(0),
            Target::Permanent(mox),
        )],
    );
    settle(&mut game);
    assert_eq!(controller_of(&game, mox), Some(PlayerId::One), "stolen");

    game.move_permanents_to_graveyard(&[dack]);
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == dack),
        "Dack is gone",
    );
    assert_eq!(
        controller_of(&game, mox),
        Some(PlayerId::One),
        "and the Mox is still yours",
    );
    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == mox)
            .map(|permanent| permanent.card.owner),
        Some(PlayerId::Two),
        "owned by them and controlled by you, which is what a theft is",
    );
}

/// Dack arrives with three loyalty, and what that buys is the whole of his
/// place in a cube: the draw-two and the theft are affordable the turn he
/// lands, the emblem is three turns of plussing away, and whichever one you
/// take is the only loyalty ability he offers that turn.
#[test]
fn three_loyalty_buys_two_of_his_three_abilities() {
    let (mut game, dack) = staged();
    // The theft wants something to point at before it is offered at all.
    game.put_onto_battlefield(PlayerId::Two, cards::MOX_SAPPHIRE)
        .expect("cataloged");
    drain_pending(&mut game);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    let offers = |game: &Game, index: usize| {
        let wanted = activated_ability_for(game, dack, index);
        game.legal_actions(PlayerId::One).into_iter().any(|action| {
            matches!(action, Action::ActivateAbility { source, ability, .. }
                    if source == dack && ability == wanted)
        })
    };

    let loyalty = |game: &Game| {
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == dack)
            .expect("he is there")
            .counters(CounterKind::Loyalty)
    };
    assert_eq!(loyalty(&game), 3, "he arrives at three");
    assert!(offers(&game, 0), "which pays the plus one");
    assert!(offers(&game, 1), "and the minus two");
    assert!(!offers(&game, 2), "but is three short of the emblem");

    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == dack)
        .expect("he is there")
        .set_counters(CounterKind::Loyalty, 6);
    assert!(offers(&game, 2), "six is what the emblem wanted");

    activate(
        &mut game,
        dack,
        0,
        vec![TargetSelection::single(
            TargetSlotId(0),
            Target::Player(PlayerId::One),
        )],
    );
    settle(&mut game);

    assert!(
        !offers(&game, 0) && !offers(&game, 1) && !offers(&game, 2),
        "and one loyalty ability a turn is all he has, however much he holds",
    );
}
