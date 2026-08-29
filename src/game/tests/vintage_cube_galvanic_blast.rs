//! Galvanic Blast: one red mana for two damage, or four in the deck that is
//! actually playing it.

use super::*;

/// Player One holding a Blast, with `artifacts` on the battlefield.
fn staged(artifacts: usize) -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    for _ in 0..artifacts {
        game.put_onto_battlefield(PlayerId::One, cards::MANIFOLD_KEY)
            .expect("cataloged");
    }
    let target = game
        .put_onto_battlefield(PlayerId::Two, cards::SERRA_ANGEL)
        .expect("cataloged");
    drain_pending(&mut game);
    let spell = game
        .build_zone(PlayerId::One, &[cards::GALVANIC_BLAST])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let spell_id = spell.id;
    game.players[0].hand.push(spell);
    game.players[1].life = 20;
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    (game, spell_id, target)
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

/// Casts it at the given target.
fn cast_at(game: &mut Game, spell: GameObjectId, target: Target) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == spell
                    && choices
                        .targets()
                        .iter()
                        .any(|slot| slot.targets().contains(&target))
            }
            _ => false,
        })
        .expect("one red mana casts it");
    game.apply(PlayerId::One, action).expect("it is cast");
    settle(game);
}

fn damage_on(game: &Game, id: GameObjectId) -> u16 {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .map_or(0, |permanent| permanent.damage)
}

/// Without metalcraft it deals two.
#[test]
fn two_artifacts_is_two_damage() {
    let (mut game, spell, angel) = staged(2);

    cast_at(&mut game, spell, Target::Permanent(angel));

    assert_eq!(damage_on(&game, angel), 2);
}

/// With three artifacts it deals four instead, which kills the 4/4.
#[test]
fn three_artifacts_is_four_damage() {
    let (mut game, spell, angel) = staged(3);

    cast_at(&mut game, spell, Target::Permanent(angel));

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == angel),
        "four damage kills a 4/4",
    );
}

/// More than three is still four: it is a threshold, not a count.
#[test]
fn four_artifacts_is_still_four_damage() {
    let (mut game, spell, _angel) = staged(4);

    cast_at(&mut game, spell, Target::Player(PlayerId::Two));

    assert_eq!(game.players[1].life, 16);
}

/// "Any target": a player takes the same two.
#[test]
fn it_can_point_at_a_player() {
    let (mut game, spell, _angel) = staged(0);

    cast_at(&mut game, spell, Target::Player(PlayerId::Two));

    assert_eq!(game.players[1].life, 18);
}

/// The count is read as it resolves, so an artifact that leaves in response
/// takes the extra two with it.
#[test]
fn losing_an_artifact_in_response_drops_it_back_to_two() {
    let (mut game, spell, angel) = staged(3);
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == spell
                    && choices
                        .targets()
                        .iter()
                        .any(|slot| slot.targets().contains(&Target::Permanent(angel)))
            }
            _ => false,
        })
        .expect("one red mana casts it");
    game.apply(PlayerId::One, action).expect("it is cast");

    // Still on the stack: one artifact leaves before it resolves.
    let doomed = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == ObjectKind::Card(cards::MANIFOLD_KEY))
        .expect("an artifact is there")
        .card
        .id;
    game.move_permanents_to_graveyard(&[doomed]);
    settle(&mut game);

    assert_eq!(damage_on(&game, angel), 2, "two artifacts is two damage");
}

/// "You control three or more artifacts", so the artifacts across the table
/// are not yours to count -- and an artifact creature of your own is.
#[test]
fn metalcraft_counts_your_artifacts_and_only_yours() {
    let (mut game, spell, _angel) = staged(2);
    for _ in 0..3 {
        game.put_onto_battlefield(PlayerId::Two, cards::MANIFOLD_KEY)
            .expect("cataloged");
    }
    drain_pending(&mut game);

    cast_at(&mut game, spell, Target::Player(PlayerId::Two));

    assert_eq!(
        game.players[1].life, 18,
        "their three artifacts left it at two damage",
    );

    let (mut game, spell, _angel) = staged(2);
    game.put_onto_battlefield(PlayerId::One, cards::ORNITHOPTER)
        .expect("cataloged");
    drain_pending(&mut game);

    cast_at(&mut game, spell, Target::Player(PlayerId::Two));

    assert_eq!(
        game.players[1].life, 16,
        "but an artifact creature of your own is the third artifact",
    );
}

/// The mirror of that: the count is read on resolution, so an artifact made
/// while the Blast is on the stack turns two damage into four -- and four is
/// what kills a Serra Angel.
#[test]
fn a_servo_made_in_response_turns_two_damage_into_four() {
    let (mut game, spell, angel) = staged(1);
    let foundry = game
        .put_onto_battlefield(PlayerId::One, cards::RETROFITTER_FOUNDRY)
        .expect("cataloged");
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    drain_pending(&mut game);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);
    game.priority = PlayerId::One;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == spell
                    && choices
                        .targets()
                        .iter()
                        .any(|slot| slot.targets().contains(&Target::Permanent(angel)))
            }
            _ => false,
        })
        .expect("one red mana casts it");
    game.apply(PlayerId::One, action).expect("it is cast");

    // Two artifacts so far; the Foundry answers at instant speed with a third.
    let make_a_servo = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source,
                ability: AbilityOrigin::Printed { ability, .. },
                ..
            } => *source == foundry && *ability == AbilityId(1),
            _ => false,
        })
        .expect("two mana and a tap while the Blast waits");
    game.apply(PlayerId::One, make_a_servo)
        .expect("it activates");
    settle(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == angel),
        "three artifacts by the time it resolved, so four damage killed the Angel",
    );
}
