//! Suplex: three damage that also takes the creature out of the graveyard it
//! was counting on, or the artifact half when there is nothing to throw.

use super::*;

const SLAM: usize = 0;
const ARTIFACT: usize = 1;

fn mode(index: usize) -> ModeId {
    ModeId::from_index(index).expect("one of the two")
}

/// Player One holding a Suplex with the mana for it, and `theirs` on the
/// battlefield under Player Two.
fn staged(theirs: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    for definition in theirs {
        game.put_onto_battlefield(PlayerId::Two, *definition)
            .expect("cataloged");
    }
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [2, 1];
    drain_pending(&mut game);
    let suplex = game
        .build_zone(PlayerId::One, &[cards::SUPLEX])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = suplex.id;
    game.players[0].hand.push(suplex);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 6);
    (game, id)
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

/// Which modes Suplex is offering, with the targets each one would take.
fn casts(game: &Game, card: GameObjectId) -> Vec<(Vec<ModeId>, Vec<Target>)> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell {
                card: id, choices, ..
            } if id == card => Some((
                choices.modes().to_vec(),
                choices.iter_targets().copied().collect(),
            )),
            _ => None,
        })
        .collect()
}

fn cast_at(game: &mut Game, card: GameObjectId, index: usize, target: GameObjectId) {
    let wanted = [mode(index)];
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell {
                card: id, choices, ..
            } => {
                *id == card
                    && choices.modes() == wanted
                    && choices
                        .iter_targets()
                        .any(|chosen| *chosen == Target::Permanent(target))
            }
            _ => false,
        })
        .unwrap_or_else(|| panic!("mode {index} is castable at that permanent"));
    game.apply(PlayerId::One, action).expect("it is cast");
    settle(game);
}

/// Puts a Bolt in Player One's hand and throws it at `target`, which is how
/// these tests kill something Suplex left standing.
fn bolt(game: &mut Game, target: GameObjectId) {
    let bolt = game
        .build_zone(PlayerId::One, &[cards::LIGHTNING_BOLT])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let bolt_id = bolt.id;
    game.players[0].hand.push(bolt);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == bolt_id
                    && choices
                        .iter_targets()
                        .any(|chosen| *chosen == Target::Permanent(target))
            }
            _ => false,
        })
        .expect("the Bolt is castable at it");
    game.apply(PlayerId::One, action).expect("it is cast");
    settle(game);
}

fn on_battlefield(game: &Game, definition: CardDefinitionId) -> bool {
    game.battlefield
        .iter()
        .any(|permanent| permanent.card.definition == definition)
}

fn in_zone(zone: &[CardInstance], definition: CardDefinitionId) -> bool {
    zone.iter().any(|card| card.definition == definition)
}

/// Carries the turn round to Player One's next precombat main phase.
fn next_turn(game: &mut Game) {
    game.advance_step();
    settle(game);
    for _ in 0..64 {
        if game.step == Step::PrecombatMain && game.active_player == PlayerId::One {
            break;
        }
        game.advance_step();
        settle(game);
    }
}

/// Three damage kills the Bears, and the exile clause takes it from there.
#[test]
fn a_creature_it_kills_is_exiled() {
    let (mut game, suplex) = staged(&[cards::GRIZZLY_BEARS]);
    let bears = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::GRIZZLY_BEARS)
        .expect("it is here")
        .card
        .id;

    cast_at(&mut game, suplex, SLAM, bears);

    assert!(!on_battlefield(&game, cards::GRIZZLY_BEARS), "it died");
    assert!(
        !in_zone(&game.players[1].graveyard, cards::GRIZZLY_BEARS),
        "and not into the graveyard"
    );
    assert!(
        in_zone(&game.players[1].exile, cards::GRIZZLY_BEARS),
        "it is exiled instead"
    );
}

/// The clause is about the creature rather than about the damage: a Spider
/// that shrugs the three off is still exiled when something else finishes it
/// later in the turn.
#[test]
fn a_creature_that_survives_is_still_exiled_when_it_dies() {
    let (mut game, suplex) = staged(&[cards::GIANT_SPIDER]);
    let spider = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::GIANT_SPIDER)
        .expect("it is here")
        .card
        .id;

    cast_at(&mut game, suplex, SLAM, spider);
    assert!(
        on_battlefield(&game, cards::GIANT_SPIDER),
        "four toughness takes three damage and lives"
    );

    bolt(&mut game, spider);

    assert!(!on_battlefield(&game, cards::GIANT_SPIDER), "six kills it");
    assert!(
        in_zone(&game.players[1].exile, cards::GIANT_SPIDER),
        "the replacement was still on it"
    );
}

/// "This turn" is the whole of it: a survivor that dies the following turn
/// goes to the graveyard like anything else.
#[test]
fn the_replacement_ends_with_the_turn() {
    let (mut game, suplex) = staged(&[cards::GIANT_SPIDER]);
    let spider = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::GIANT_SPIDER)
        .expect("it is here")
        .card
        .id;

    cast_at(&mut game, suplex, SLAM, spider);
    next_turn(&mut game);
    // The damage wore off in cleanup, so it takes the full four again.
    bolt(&mut game, spider);
    bolt(&mut game, spider);

    assert!(!on_battlefield(&game, cards::GIANT_SPIDER), "six kills it");
    assert!(
        in_zone(&game.players[1].graveyard, cards::GIANT_SPIDER),
        "a turn later it dies the ordinary way"
    );
}

/// The other half exiles an artifact outright.
#[test]
fn the_second_mode_exiles_an_artifact() {
    let (mut game, suplex) = staged(&[cards::SOL_RING]);
    let ring = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::SOL_RING)
        .expect("it is here")
        .card
        .id;

    cast_at(&mut game, suplex, ARTIFACT, ring);

    assert!(!on_battlefield(&game, cards::SOL_RING), "the Ring is gone");
    assert!(
        in_zone(&game.players[1].exile, cards::SOL_RING),
        "exiled rather than destroyed"
    );
}

/// One mode, and only the modes with something to point at. A board of
/// creatures offers the slam alone.
#[test]
fn only_the_mode_with_a_legal_target_is_offered() {
    let (game, suplex) = staged(&[cards::GRIZZLY_BEARS]);

    let offered = casts(&game, suplex);
    assert!(!offered.is_empty(), "it is castable");
    assert!(
        offered
            .iter()
            .all(|(modes, _)| modes.as_slice() == [mode(SLAM)]),
        "no artifact means no artifact mode: {offered:?}"
    );
}

/// Its ruling: "the replacement effect will exile the target creature if it
/// would die this turn for any reason, not just due to lethal damage." A
/// Wrath of God is a reason.
#[test]
fn a_slammed_creature_destroyed_by_something_else_is_exiled_too() {
    let (mut game, suplex) = staged(&[cards::GIANT_SPIDER]);
    let spider = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::GIANT_SPIDER)
        .expect("it is here")
        .card
        .id;

    cast_at(&mut game, suplex, SLAM, spider);
    assert!(
        on_battlefield(&game, cards::GIANT_SPIDER),
        "four toughness takes three damage and lives",
    );

    let wrath = game
        .build_zone(PlayerId::One, &[cards::WRATH_OF_GOD])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let wrath_id = wrath.id;
    game.players[0].hand.push(wrath);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 2);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == wrath_id))
        .expect("four mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game);

    assert!(
        !on_battlefield(&game, cards::GIANT_SPIDER),
        "the sweeper destroyed it",
    );
    assert!(
        in_zone(&game.players[1].exile, cards::GIANT_SPIDER),
        "and the replacement sent it to exile rather than the graveyard",
    );
    assert!(
        !in_zone(&game.players[1].graveyard, cards::GIANT_SPIDER),
        "which is the whole of what the mode buys",
    );
}

/// Neither mode says whose permanent it has to be. Your own creature is a
/// legal target, and the replacement rides along with it into your exile.
#[test]
fn it_may_be_aimed_at_your_own_board() {
    let (mut game, suplex) = staged(&[]);
    game.put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    let bears = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::GRIZZLY_BEARS)
        .expect("it is here")
        .card
        .id;

    cast_at(&mut game, suplex, SLAM, bears);

    assert!(!on_battlefield(&game, cards::GRIZZLY_BEARS), "it died");
    assert!(
        in_zone(&game.players[0].exile, cards::GRIZZLY_BEARS),
        "into the exile of the player who owned it"
    );
}

/// The artifact mode is just as indiscriminate: your own Sol Ring is a legal
/// target for it.
#[test]
fn the_artifact_mode_will_take_your_own_artifact() {
    let (mut game, suplex) = staged(&[]);
    game.put_onto_battlefield(PlayerId::One, cards::SOL_RING)
        .expect("cataloged");
    let ring = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::SOL_RING)
        .expect("it is here")
        .card
        .id;

    cast_at(&mut game, suplex, ARTIFACT, ring);

    assert!(!on_battlefield(&game, cards::SOL_RING), "the Ring is gone");
    assert!(
        in_zone(&game.players[0].exile, cards::SOL_RING),
        "exiled from under its own controller"
    );
}

/// The replacement is on the one creature the spell targeted. Everything else
/// that dies in the same sweeper still dies the ordinary way.
#[test]
fn only_the_slammed_creature_is_exiled_by_the_sweeper() {
    let (mut game, suplex) = staged(&[cards::GIANT_SPIDER, cards::GRIZZLY_BEARS]);
    let spider = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::GIANT_SPIDER)
        .expect("it is here")
        .card
        .id;

    cast_at(&mut game, suplex, SLAM, spider);

    let wrath = game
        .build_zone(PlayerId::One, &[cards::WRATH_OF_GOD])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let wrath_id = wrath.id;
    game.players[0].hand.push(wrath);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 2);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == wrath_id))
        .expect("four mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game);

    assert!(
        in_zone(&game.players[1].exile, cards::GIANT_SPIDER),
        "the target the Suplex marked went to exile"
    );
    assert!(
        in_zone(&game.players[1].graveyard, cards::GRIZZLY_BEARS),
        "the bystander went to the graveyard"
    );
}

/// Both halves at once: with a creature and an artifact across the table,
/// each mode is on offer with its own targets, and choosing is the whole of
/// what "choose one" asks.
#[test]
fn both_modes_are_offered_when_both_have_a_target() {
    let (game, suplex) = staged(&[cards::GRIZZLY_BEARS, cards::HOWLING_MINE]);

    let offered = casts(&game, suplex);
    assert!(
        offered
            .iter()
            .any(|(modes, _)| modes.as_slice() == [mode(SLAM)]),
        "the slam is there: {offered:?}",
    );
    assert!(
        offered
            .iter()
            .any(|(modes, _)| modes.as_slice() == [mode(ARTIFACT)]),
        "and so is the artifact half: {offered:?}",
    );
    assert!(
        offered.iter().all(|(modes, _)| modes.len() == 1),
        "one of them, never both: {offered:?}",
    );
}

/// "If that creature *would die* this turn" is a death and not a damaging.
/// An indestructible Myr takes the three, shrugs off a Wrath after it, and
/// is never exiled, because nothing ever killed it.
#[test]
fn an_indestructible_creature_never_dies_and_so_is_never_exiled() {
    let (mut game, suplex) = staged(&[cards::DARKSTEEL_MYR]);
    let myr = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::DARKSTEEL_MYR)
        .expect("it is out")
        .card
        .id;

    cast_at(&mut game, suplex, SLAM, myr);

    assert!(
        on_battlefield(&game, cards::DARKSTEEL_MYR),
        "three damage is not lethal to what cannot be destroyed",
    );

    let wrath = card(190_900, cards::WRATH_OF_GOD, PlayerId::One);
    let wrath_id = wrath.id;
    game.players[0].hand.push(wrath);
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 4);
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == wrath_id))
        .expect("four white casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game);

    assert!(
        on_battlefield(&game, cards::DARKSTEEL_MYR),
        "and a Wrath is no answer to it either",
    );
    assert!(
        !in_zone(&game.players[1].exile, cards::DARKSTEEL_MYR),
        "so the replacement never had a death to replace",
    );
}

/// It is a sorcery: their turn is no time for it, whatever is standing there
/// to point at.
#[test]
fn it_waits_for_your_own_main_phase() {
    let (mut game, suplex) = staged(&[cards::GRIZZLY_BEARS]);
    assert!(
        !casts(&game, suplex).is_empty(),
        "your main phase is its window"
    );

    game.active_player = PlayerId::Two;
    assert!(casts(&game, suplex).is_empty(), "and their turn is not");
}
