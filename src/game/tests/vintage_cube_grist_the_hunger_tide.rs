//! Grist, the Hunger Tide: a body every turn off the top of the library,
//! and an answer on the turn he lands.

use super::*;

/// Grist on the battlefield with three loyalty, and `library` on top of
/// Player One's library in the order given -- the last is the top card.
fn staged(library: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    game.players[0].graveyard.clear();
    for (index, definition) in library.iter().enumerate() {
        let card = game
            .build_zone(PlayerId::One, &[*definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        let _ = index;
        game.players[0].library.push(card);
    }
    let grist = game
        .put_onto_battlefield(PlayerId::One, cards::GRIST_THE_HUNGER_TIDE)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, grist)
}

fn settle(game: &mut Game) {
    for _ in 0..32 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            // An optional payment offers declining first; this one always
            // pays, which is what the sacrifice is for.
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
            .expect("the offered choice is legal");
            continue;
        }
        if game.stack.is_empty()
            && game.pending_triggers.is_empty()
            && game.pending_decisions.is_empty()
        {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    drain_pending(game);
    game.check_state_based_actions();
}

/// Activates Grist's printed ability `index` and lets it resolve. `wanted`
/// names the targets to prefer where the ability takes any.
fn activate(game: &mut Game, grist: GameObjectId, index: u8, wanted: &[GameObjectId]) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source,
                ability: AbilityOrigin::Printed { ability, .. },
                targets,
                ..
            } => {
                *source == grist
                    && *ability == AbilityId(index)
                    && (wanted.is_empty()
                        || targets.iter().all(|selection| {
                            selection.targets().iter().all(|target| {
                                matches!(target, Target::Permanent(id) if wanted.contains(id))
                            })
                        }))
            }
            _ => false,
        })
        .expect("the loyalty ability is offered");
    game.apply(PlayerId::One, action).expect("it is activated");
    settle(game);
}

fn loyalty(game: &Game, grist: GameObjectId) -> u16 {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == grist)
        .expect("he is there")
        .counters(CounterKind::Loyalty)
}

fn insects(game: &Game) -> usize {
    game.battlefield
        .iter()
        .filter(|permanent| game.effective_subtypes(permanent).contains(&"Insect"))
        .count()
}

/// A non-Insect on top: one token, one card milled, and the loyalty is the
/// plus and nothing more.
#[test]
fn a_plain_top_card_makes_one_insect() {
    let (mut game, grist) = staged(&[cards::ISLAND, cards::GRIZZLY_BEARS]);

    activate(&mut game, grist, 1, &[]);

    assert_eq!(insects(&game), 1, "one Insect");
    assert_eq!(game.players[0].graveyard.len(), 1, "one card milled");
    assert_eq!(
        game.players[0].graveyard[0].definition,
        cards::GRIZZLY_BEARS,
        "the top card and no more",
    );
    assert_eq!(loyalty(&game, grist), 4, "three plus the activation");
}

/// An Insect on top keeps the process going: two tokens, two cards milled,
/// and a loyalty counter for the Insect on top of the plus.
#[test]
fn an_insect_repeats_the_process() {
    let (mut game, grist) = staged(&[cards::ISLAND, cards::GRIZZLY_BEARS, cards::BOND_BEETLE]);
    // The Beetle on top is an Insect; the bear under it is not.
    activate(&mut game, grist, 1, &[]);

    assert_eq!(insects(&game), 2, "a token for each pass");
    assert_eq!(game.players[0].graveyard.len(), 2, "two cards milled");
    assert_eq!(loyalty(&game, grist), 5, "the plus and the Insect");
}

/// An empty library ends the process after the token it already made.
#[test]
fn an_empty_library_still_makes_the_insect() {
    let (mut game, grist) = staged(&[]);

    activate(&mut game, grist, 1, &[]);

    assert_eq!(insects(&game), 1, "the token comes before the mill");
    assert!(game.players[0].graveyard.is_empty(), "nothing to mill");
    assert_eq!(loyalty(&game, grist), 4);
}

/// The minus sacrifices a creature and destroys something with it.
#[test]
fn the_minus_trades_a_creature_for_theirs() {
    let (mut game, grist) = staged(&[cards::ISLAND]);
    let mine = game
        .put_onto_battlefield(PlayerId::One, cards::SAVANNAH_LIONS)
        .expect("cataloged");
    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::GRAVE_TITAN)
        .expect("cataloged");
    drain_pending(&mut game);

    activate(&mut game, grist, 2, &[theirs]);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == theirs),
        "their creature is destroyed",
    );
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == mine),
        "and yours paid for it",
    );
    assert_eq!(loyalty(&game, grist), 1, "three minus two");
}

/// The ultimate drains for the creature cards in your graveyard -- and
/// paying it puts Grist himself among them, because a Grist that is not on
/// the battlefield is a creature card too.
#[test]
fn the_ultimate_drains_for_the_graveyard() {
    let (mut game, grist) = staged(&[cards::ISLAND]);
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == grist)
        .expect("he is there")
        .set_counters(CounterKind::Loyalty, 5);
    for index in 0..3 {
        game.players[0]
            .graveyard
            .push(card(108_000 + index, cards::GRIZZLY_BEARS, PlayerId::One));
    }
    game.players[0]
        .graveyard
        .push(card(108_100, cards::ISLAND, PlayerId::One));
    game.players[1].life = 20;

    activate(&mut game, grist, 3, &[]);

    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::GRIST_THE_HUNGER_TIDE),
        "spending the last loyalty put him there",
    );
    assert_eq!(
        game.players[1].life, 16,
        "three Bears and Grist, and the land is not one of them",
    );
}

/// "As long as Grist isn't on the battlefield, it's a 1/1 Insect creature":
/// a Grist milled off the top is an Insect card, so the process keeps going
/// the way it would for any other Insect.
#[test]
fn a_grist_on_top_is_an_insect_and_repeats_the_process() {
    let (mut game, grist) = staged(&[
        cards::ISLAND,
        cards::GRIZZLY_BEARS,
        cards::GRIST_THE_HUNGER_TIDE,
    ]);

    activate(&mut game, grist, 1, &[]);

    assert_eq!(insects(&game), 2, "the Grist milled counted as an Insect");
    assert_eq!(game.players[0].graveyard.len(), 2, "two cards milled");
    assert_eq!(loyalty(&game, grist), 5, "the plus and the Insect");
}

/// He is one wherever he is not on the battlefield -- a hand and a
/// graveyard alike -- and he is not one while he is.
#[test]
fn he_is_a_creature_card_off_the_battlefield_only() {
    let (mut game, grist) = staged(&[]);
    let is_a_creature_card = |game: &Game, card: &CardInstance, zone| {
        game.card_object_matches(
            ObjectPredicateDef::HasType(CardType::Creature),
            card,
            zone,
            grist,
        )
    };

    let in_hand = game
        .build_zone(PlayerId::One, &[cards::GRIST_THE_HUNGER_TIDE])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    assert!(
        is_a_creature_card(&game, &in_hand, ZoneKind::Hand),
        "a Grist in hand is a creature card",
    );
    game.players[0].graveyard.push(in_hand);
    let buried = game.players[0].graveyard.last().expect("it is there");
    assert!(
        is_a_creature_card(&game, buried, ZoneKind::Graveyard),
        "and so is one in a graveyard",
    );

    let permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == grist)
        .expect("he is on the battlefield");
    assert!(
        !game
            .permanent_types(permanent)
            .is_some_and(|types| types.contains(CardType::Creature)),
        "the one on the battlefield is a planeswalker and nothing else",
    );
}

/// The body travels with the type: a Grist outside the battlefield is a
/// 1/1, whatever a predicate asks of it.
#[test]
fn the_card_off_the_battlefield_is_a_one_one() {
    let (mut game, _grist) = staged(&[]);
    let card = game
        .build_zone(PlayerId::One, &[cards::GRIST_THE_HUNGER_TIDE])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = card.id;
    game.players[0].graveyard.push(card);

    assert_eq!(game.current_or_last_known_power(id), Some(1));
    assert_eq!(game.current_or_last_known_toughness(id), Some(1));
}

/// "You may sacrifice a creature": declining keeps the creature and the
/// destruction both, and the loyalty is spent either way.
#[test]
fn declining_the_sacrifice_destroys_nothing() {
    let (mut game, grist) = staged(&[cards::ISLAND]);
    let mine = game
        .put_onto_battlefield(PlayerId::One, cards::SAVANNAH_LIONS)
        .expect("cataloged");
    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::GRAVE_TITAN)
        .expect("cataloged");
    drain_pending(&mut game);

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source,
                ability: AbilityOrigin::Printed { ability, .. },
                targets,
                ..
            } => {
                *source == grist
                    && *ability == AbilityId(2)
                    && targets
                        .iter()
                        .any(|selection| selection.targets() == [Target::Permanent(theirs)])
            }
            _ => false,
        })
        .expect("the minus is offered");
    game.apply(PlayerId::One, action).expect("it is activated");

    // The payment asks as the ability resolves, and this time the answer is
    // no.
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
        .expect("the optional sacrifice asks");
    let decline = decision
        .options
        .iter()
        .find(|option| option.label == "Decline")
        .expect("declining is one of the answers")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![decline],
        },
    )
    .expect("declining is legal");
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == mine),
        "your creature is still there",
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == theirs),
        "and so is theirs: no sacrifice, no destruction",
    );
    assert_eq!(loyalty(&game, grist), 1, "the loyalty was paid regardless");
}

/// The deviation this card carries, pinned: the target is named as the minus
/// is activated rather than by the printed reflexive trigger, so answering
/// that target counters the whole ability and the creature that would have
/// been sacrificed is still standing. On the printed card the sacrifice is
/// offered anyway and only the destruction is lost.
#[test]
fn an_answered_target_saves_the_sacrifice_too() {
    let (mut game, grist) = staged(&[cards::ISLAND]);
    let mine = game
        .put_onto_battlefield(PlayerId::One, cards::SAVANNAH_LIONS)
        .expect("cataloged");
    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::GRAVE_TITAN)
        .expect("cataloged");
    drain_pending(&mut game);

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source,
                ability: AbilityOrigin::Printed { ability, .. },
                targets,
                ..
            } => {
                *source == grist
                    && *ability == AbilityId(2)
                    && targets
                        .iter()
                        .any(|selection| selection.targets() == [Target::Permanent(theirs)])
            }
            _ => false,
        })
        .expect("the minus is offered");
    game.apply(PlayerId::One, action).expect("it is activated");
    game.move_permanents_to_graveyard(&[theirs]);
    settle(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == mine),
        "nothing was sacrificed, because the ability never resolved",
    );
    assert_eq!(loyalty(&game, grist), 1, "the loyalty was still paid");
}
