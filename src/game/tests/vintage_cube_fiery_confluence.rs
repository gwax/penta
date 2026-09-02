//! Fiery Confluence: three modes from a list of three, taken in any mixture,
//! and the same one as many times as you like.

use super::*;

/// Player One holding a Confluence with the mana to cast it.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let confluence = game
        .build_zone(PlayerId::One, &[cards::FIERY_CONFLUENCE])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = confluence.id;
    game.players[0].hand.push(confluence);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 4);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, id)
}

fn cast_with(game: &mut Game, card: GameObjectId, wanted: &[ModeId], targets: &[Target]) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell {
                card: id, choices, ..
            } => {
                *id == card
                    && choices.modes() == wanted
                    && choices.iter_targets().copied().collect::<Vec<_>>() == targets
            }
            _ => false,
        })
        .expect("that combination of modes and targets is on offer");
    game.apply(PlayerId::One, action).expect("it is castable");
    settle(game);
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
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
    game.check_state_based_actions();
}

const SWEEP: ModeId = ModeId(0);
const BURN: ModeId = ModeId(1);
const SHATTER: ModeId = ModeId(2);

/// Three burn modes is six damage to the other player.
#[test]
fn three_burns_is_six_to_the_face() {
    let (mut game, confluence) = staged();
    game.players[1].life = 20;

    cast_with(&mut game, confluence, &[BURN, BURN, BURN], &[]);

    assert_eq!(game.players[1].life, 14);
}

/// Three sweeps is three damage to every creature, including your own.
#[test]
fn three_sweeps_is_three_to_every_creature() {
    let (mut game, confluence) = staged();
    let mine = creature(170_000, cards::GRIZZLY_BEARS, PlayerId::One);
    let mine_id = mine.card.id;
    game.battlefield.push(mine);
    let theirs = creature(170_001, cards::SERRA_ANGEL, PlayerId::Two);
    let theirs_id = theirs.card.id;
    game.battlefield.push(theirs);

    cast_with(&mut game, confluence, &[SWEEP, SWEEP, SWEEP], &[]);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == mine_id),
        "a 2/2 of your own dies to your own sweeper",
    );
    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == theirs_id)
            .expect("a 4/4 survives three")
            .damage,
        3,
    );
}

/// A mode that targets gets a target slot per copy chosen, so two artifacts
/// can go at once. The chosen modes come back in printed order however they
/// were picked.
#[test]
fn two_shatters_name_two_artifacts() {
    let (mut game, confluence) = staged();
    let first = game
        .put_onto_battlefield(PlayerId::Two, cards::MANIFOLD_KEY)
        .expect("cataloged");
    let second = game
        .put_onto_battlefield(PlayerId::Two, cards::GUARDIAN_IDOL)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    cast_with(
        &mut game,
        confluence,
        &[BURN, SHATTER, SHATTER],
        &[Target::Permanent(first), Target::Permanent(second)],
    );

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == first || permanent.card.id == second),
        "both named artifacts are destroyed",
    );
    assert_eq!(game.players[1].life, 18, "and the third mode still burned");
}

/// Mixing modes is the ordinary case: a sweep, a burn, and a shatter.
#[test]
fn the_three_modes_mix() {
    let (mut game, confluence) = staged();
    let bears = creature(170_100, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    let idol = game
        .put_onto_battlefield(PlayerId::Two, cards::GUARDIAN_IDOL)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    cast_with(
        &mut game,
        confluence,
        &[SWEEP, BURN, SHATTER],
        &[Target::Permanent(idol)],
    );

    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == bears_id)
            .expect("a 2/2 survives one")
            .damage,
        1,
    );
    assert_eq!(game.players[1].life, 18);
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == idol),
    );
}

/// "If all targets for the chosen modes become illegal before the Confluence
/// resolves, the spell won't resolve and none of its effects will happen."
/// Two burn modes and a shatter is still one spell: cracking the artifact it
/// named in response saves the four damage as well.
#[test]
fn losing_its_only_target_stops_the_whole_spell() {
    let (mut game, confluence) = staged();
    let lotus = game
        .put_onto_battlefield(PlayerId::Two, cards::BLACK_LOTUS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell {
                card: id, choices, ..
            } => {
                *id == confluence
                    && choices.modes() == [BURN, BURN, SHATTER]
                    && choices.iter_targets().copied().collect::<Vec<_>>()
                        == [Target::Permanent(lotus)]
            }
            _ => false,
        })
        .expect("two burns and a shatter at the Lotus");
    game.apply(PlayerId::One, cast).expect("it is castable");

    // They hold priority and crack the Lotus for mana rather than let it be
    // shattered, which takes the spell's only target with it.
    game.priority = PlayerId::Two;
    let crack = Action::ActivateManaAbility {
        source: lotus,
        ability: mana_ability_for(&game, lotus, ManaColor::Blue),
        color: ManaColor::Blue,
        counters_removed: None,
        cost_object: None,
        combination: None,
        triggered_mana: None,
    };
    game.apply(PlayerId::Two, crack)
        .expect("its own ability sacrifices it");
    settle(&mut game);

    assert!(
        game.stack.is_empty(),
        "the Confluence left the stack one way or another",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::FIERY_CONFLUENCE),
        "countered on resolution for having no legal targets",
    );
    assert_eq!(
        game.players[1].life, 20,
        "so the two burn modes never happened either",
    );
}

/// The other half of the same ruling: with one target still legal the spell
/// resolves, does nothing to the one that got away, and carries out every
/// mode that needed no target.
#[test]
fn one_surviving_target_still_resolves_the_rest() {
    let (mut game, confluence) = staged();
    let lotus = game
        .put_onto_battlefield(PlayerId::Two, cards::BLACK_LOTUS)
        .expect("cataloged");
    let key = game
        .put_onto_battlefield(PlayerId::Two, cards::MANIFOLD_KEY)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell {
                card: id, choices, ..
            } => {
                *id == confluence
                    && choices.modes() == [BURN, SHATTER, SHATTER]
                    && choices.iter_targets().copied().collect::<Vec<_>>()
                        == [Target::Permanent(lotus), Target::Permanent(key)]
            }
            _ => false,
        })
        .expect("a burn and both artifacts named");
    game.apply(PlayerId::One, cast).expect("it is castable");

    game.priority = PlayerId::Two;
    let crack = Action::ActivateManaAbility {
        source: lotus,
        ability: mana_ability_for(&game, lotus, ManaColor::Blue),
        color: ManaColor::Blue,
        counters_removed: None,
        cost_object: None,
        combination: None,
        triggered_mana: None,
    };
    game.apply(PlayerId::Two, crack)
        .expect("its own ability sacrifices it");
    settle(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == key),
        "the target that was still there is destroyed",
    );
    assert_eq!(game.players[1].life, 18, "and the burn mode still landed");
}

/// "If a mode requires a target, you can select that mode only if there's a
/// legal target available." With no artifact anywhere, the shatter is not
/// on the menu and the other two still are.
#[test]
fn the_shatter_is_not_offered_without_an_artifact() {
    let (game, confluence) = staged();

    let offered = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell { card, choices, .. } if card == confluence => {
                Some(choices.modes().to_vec())
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(!offered.is_empty(), "the spell is castable");
    assert!(
        offered.iter().all(|modes| !modes.contains(&SHATTER)),
        "nothing to destroy, so the mode cannot be taken: {offered:?}",
    );
    assert!(
        offered.iter().any(|modes| modes == &[SWEEP, BURN, BURN]),
        "and the untargeted modes are still free to mix: {offered:?}",
    );
}

/// "Each time you select that mode, you can choose a different target, or
/// you can choose the same target." The second copy finds the artifact
/// already gone and does nothing, which is the price of naming it twice.
#[test]
fn two_shatters_may_name_the_same_artifact_twice() {
    let (mut game, confluence) = staged();
    let key = game
        .put_onto_battlefield(PlayerId::Two, cards::MANIFOLD_KEY)
        .expect("cataloged");
    let idol = game
        .put_onto_battlefield(PlayerId::Two, cards::GUARDIAN_IDOL)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    cast_with(
        &mut game,
        confluence,
        &[BURN, SHATTER, SHATTER],
        &[Target::Permanent(key), Target::Permanent(key)],
    );

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == key),
        "the Key is destroyed",
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == idol),
        "and the Idol that was never named is untouched",
    );
}

/// "If the first or second modes are chosen multiple times, each of those
/// modes represents a separate damage-dealing event." Three burns are three
/// events of two, not one of six -- which is what anything preventing a
/// point per source would read.
#[test]
fn three_burns_are_three_separate_damage_events() {
    let (mut game, confluence) = staged();
    game.players[1].life = 20;
    let before = game.events.len();

    cast_with(&mut game, confluence, &[BURN, BURN, BURN], &[]);

    let dealt = game.events[before..]
        .iter()
        .filter_map(|event| match event {
            GameEvent::DamageDealt {
                player: PlayerId::Two,
                amount,
            } => Some(*amount),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        dealt,
        vec![2, 2, 2],
        "three deliveries of two rather than one of six",
    );
    assert_eq!(game.players[1].life, 14, "and six life all the same");
}

/// "If a Confluence is copied, the effect that creates the copy will usually
/// allow you to choose new targets, but you can't choose new modes." A Fork
/// takes the two burns and the shatter as they were chosen -- and the burns
/// read "each opponent" from the copy's own controller, so they turn around
/// and point at the player who cast the Confluence.
#[test]
fn a_forked_confluence_keeps_its_modes_and_may_move_its_target() {
    let (mut game, confluence) = staged();
    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::MIND_STONE)
        .expect("cataloged");
    let mine = game
        .put_onto_battlefield(PlayerId::One, cards::MIND_STONE)
        .expect("cataloged");
    drain_pending(&mut game);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == confluence
                    && choices.modes() == [BURN, BURN, SHATTER]
                    && choices.iter_targets().copied().collect::<Vec<_>>()
                        == [Target::Permanent(theirs)]
            }
            _ => false,
        })
        .expect("two burns and a shatter at their artifact");
    game.apply(PlayerId::One, cast).expect("it is cast");
    let original = game.stack.last().expect("it is on the stack").id;
    game.apply(PlayerId::One, Action::PassPriority)
        .expect("the other player gets a window before it resolves");

    let fork = card(96_100, cards::FORK, PlayerId::Two);
    let fork_id = fork.id;
    game.players[1].hand.push(fork);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Red, 2);
    let cast_fork = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == fork_id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Spell(original))
            }
            _ => false,
        })
        .expect("Fork copies a spell on the stack");
    game.apply(PlayerId::Two, cast_fork).expect("it is cast");

    // The copy's targets are the Fork controller's to move; its modes are not
    // theirs to touch, so the one question asked is where the shatter points.
    let retarget = loop {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            break decision;
        }
        let priority = game.priority;
        game.apply(priority, Action::PassPriority)
            .expect("the Fork is resolving");
    };
    assert_eq!(retarget.prompt, "Choose targets for the copy");
    let moved = retarget
        .options
        .iter()
        .find(|option| option.label != "Keep original targets")
        .expect("moving it is one of the answers")
        .id;
    game.apply(
        retarget.player,
        Action::ChooseDecision {
            decision: retarget.id,
            options: vec![moved],
        },
    )
    .expect("the copy may be pointed somewhere else");
    settle(&mut game);

    assert_eq!(
        game.players[0].life, 16,
        "the copy's two burns came back at the Confluence's caster",
    );
    assert_eq!(
        game.players[1].life, 16,
        "and the original's two hit the Fork's caster",
    );
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == theirs || permanent.card.id == mine),
        "one shatter each, and the copy's was moved to the other artifact",
    );
}
