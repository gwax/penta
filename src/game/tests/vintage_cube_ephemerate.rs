//! Ephemerate: one white mana for two enter triggers, a turn apart.

use super::*;

/// Player One holding an Ephemerate, with a creature out that notices
/// arriving.
fn staged() -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    game.players[0].exile.clear();
    let blinked = game
        .put_onto_battlefield(PlayerId::One, cards::SNAPCASTER_MAGE)
        .expect("cataloged");
    drain_pending(&mut game);
    settle(&mut game);
    let spell = game
        .build_zone(PlayerId::One, &[cards::EPHEMERATE])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let spell_id = spell.id;
    game.players[0].hand.push(spell);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 2);
    (game, spell_id, blinked)
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

fn cast(game: &mut Game, spell: GameObjectId, target: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == spell
                    && choices
                        .targets()
                        .iter()
                        .any(|slot| slot.targets().contains(&Target::Permanent(target)))
            }
            _ => false,
        })
        .expect("one white mana casts it at the creature");
    game.apply(PlayerId::One, action).expect("it is cast");
    settle(game);
}

/// The creature that came back, whichever object it is now.
fn blinked_creature(game: &Game) -> &Permanent {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == ObjectKind::Card(cards::SNAPCASTER_MAGE))
        .expect("it came back")
}

/// It blinks the creature: a new object, still on the battlefield, still
/// under its owner's control.
#[test]
fn it_blinks_the_creature() {
    let (mut game, spell, blinked) = staged();

    cast(&mut game, spell, blinked);

    let returned = blinked_creature(&game);
    assert_ne!(returned.card.id, blinked, "it is a new object");
    assert_eq!(returned.controller, PlayerId::One);
    assert_eq!(
        game.battlefield
            .iter()
            .filter(|permanent| {
                permanent.card.definition == ObjectKind::Card(cards::SNAPCASTER_MAGE)
            })
            .count(),
        1,
        "one of it, not two",
    );
}

/// Cast from hand it is exiled rather than buried, which is the first half
/// of rebound.
#[test]
fn cast_from_hand_it_exiles_itself() {
    let (mut game, spell, blinked) = staged();

    cast(&mut game, spell, blinked);

    assert!(
        game.players[0]
            .exile
            .iter()
            .any(|card| card.definition == cards::EPHEMERATE),
        "it is in exile",
    );
    assert!(
        !game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::EPHEMERATE),
        "and not in the graveyard",
    );
}

/// Advances to this player's next upkeep and lets the rebound trigger
/// resolve, stopping at the offer it puts up rather than answering it.
fn to_the_rebound_offer(game: &mut Game) {
    for _ in 0..60 {
        if !game.pending_decisions.is_empty() {
            return;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            game.advance_step();
            continue;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            return;
        }
    }
}

/// The second half: at the next upkeep it is offered back for nothing.
#[test]
fn it_comes_back_at_your_next_upkeep() {
    let (mut game, spell, blinked) = staged();
    cast(&mut game, spell, blinked);
    let exiled = game.players[0]
        .exile
        .iter()
        .find(|card| card.definition == cards::EPHEMERATE)
        .expect("it is in exile")
        .id;

    to_the_rebound_offer(&mut game);

    assert_eq!(game.step, Step::Upkeep, "at an upkeep");
    assert_eq!(game.active_player, PlayerId::One, "and it is yours");
    assert!(
        game.pending_decisions.iter().any(|pending| {
            pending
                .observation
                .options
                .iter()
                .any(|option| option.card.is_some_and(|(object, _)| object == exiled))
        }),
        "the exiled card is offered back",
    );
    assert!(
        game.legal_actions(PlayerId::One)
            .into_iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if card == exiled)),
        "and taking the offer is a free cast",
    );
}

/// The rebounded cast is not from hand, so nothing rebounds a second time:
/// it goes to the graveyard.
#[test]
fn the_rebounded_cast_is_not_exiled_again() {
    let (mut game, spell, blinked) = staged();
    cast(&mut game, spell, blinked);
    let exiled = game.players[0]
        .exile
        .iter()
        .find(|card| card.definition == cards::EPHEMERATE)
        .expect("it is in exile")
        .id;
    to_the_rebound_offer(&mut game);

    let take = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == exiled))
        .expect("the offer is standing");
    game.apply(PlayerId::One, take).expect("it costs nothing");
    settle(&mut game);

    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::EPHEMERATE),
        "buried rather than exiled again",
    );
    assert!(
        !game.players[0]
            .exile
            .iter()
            .any(|card| card.definition == cards::EPHEMERATE),
    );
}

/// And it blinks again on the way through, which is the whole point of
/// paying one mana for it.
#[test]
fn the_rebounded_cast_blinks_again() {
    let (mut game, spell, blinked) = staged();
    cast(&mut game, spell, blinked);
    let after_first = blinked_creature(&game).card.id;
    let exiled = game.players[0]
        .exile
        .iter()
        .find(|card| card.definition == cards::EPHEMERATE)
        .expect("it is in exile")
        .id;
    to_the_rebound_offer(&mut game);

    let take = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == exiled))
        .expect("the offer is standing");
    game.apply(PlayerId::One, take).expect("it costs nothing");
    settle(&mut game);

    let returned = blinked_creature(&game);
    assert_ne!(returned.card.id, after_first, "blinked a second time");
}

/// "Any counters on the exiled creature will cease to exist. Equipment
/// attached to it will become unattached and remain on the battlefield."
/// What comes back is a new object that remembers none of it.
#[test]
fn the_counters_go_and_the_equipment_falls_off() {
    let (mut game, spell, _snapcaster) = staged();
    let bears = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    let clamp = game
        .put_onto_battlefield(PlayerId::One, cards::SKULLCLAMP)
        .expect("cataloged");
    drain_pending(&mut game);
    settle(&mut game);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    let equip = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } => {
                *source == clamp
                    && targets
                        .iter()
                        .any(|selection| selection.targets() == [Target::Permanent(bears)])
            }
            _ => false,
        })
        .expect("equip is offered for that creature");
    game.apply(PlayerId::One, equip).expect("it equips");
    settle(&mut game);
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == bears)
        .expect("it is on the battlefield")
        .add_counters(CounterKind::PlusOnePlusOne, 2);
    game.priority = PlayerId::One;

    cast(&mut game, spell, bears);

    let returned = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == ObjectKind::Card(cards::GRIZZLY_BEARS))
        .expect("the Bears came back");
    assert_ne!(returned.card.id, bears, "a new object came back");
    assert_eq!(
        returned.counters(CounterKind::PlusOnePlusOne),
        0,
        "the counters did not travel with it",
    );
    let clamp = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == clamp)
        .expect("the Skullclamp is still on the battlefield");
    assert!(
        clamp.attached_to.is_none(),
        "on nobody: what it was on is gone",
    );
}

/// "If a token is exiled this way, it will cease to exist and won't return
/// to the battlefield."
#[test]
fn a_token_it_blinks_never_comes_back() {
    let (mut game, spell, _blinked) = staged();
    game.put_onto_battlefield(PlayerId::One, cards::ESIKA_S_CHARIOT)
        .expect("cataloged");
    drain_pending(&mut game);
    settle(&mut game);
    let cat = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == ObjectKind::Token)
        .expect("the Chariot brought its Cats")
        .card
        .id;
    let tokens_before = game
        .battlefield
        .iter()
        .filter(|permanent| permanent.card.definition == ObjectKind::Token)
        .count();
    game.priority = PlayerId::One;

    cast(&mut game, spell, cat);

    assert_eq!(
        game.battlefield
            .iter()
            .filter(|permanent| permanent.card.definition == ObjectKind::Token)
            .count(),
        tokens_before - 1,
        "the Cat left and nothing came back in its place",
    );
    assert!(
        game.players[0].exile.iter().all(|card| card.id != cat),
        "and it is not sitting in exile either",
    );
}

/// "If a spell with rebound that you cast from your hand is countered, none
/// of its effects will happen, including rebound. The spell will be put into
/// its owner's graveyard and you won't get to cast it again on your next
/// turn." Rebound is a replacement on the spell finishing resolution, and a
/// countered spell never finishes resolving.
#[test]
fn a_countered_ephemerate_goes_to_the_graveyard_and_never_rebounds() {
    let (mut game, spell, blinked) = staged();
    game.players[1]
        .hand
        .push(card(98_400, cards::COUNTERSPELL, PlayerId::Two));
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Blue, 2);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == spell
                    && choices
                        .targets()
                        .iter()
                        .any(|slot| slot.targets().contains(&Target::Permanent(blinked)))
            }
            _ => false,
        })
        .expect("one white mana casts it at the creature");
    game.apply(PlayerId::One, cast).expect("it is cast");

    game.priority = PlayerId::Two;
    let counter = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, .. } if *card == CardInstanceId(98_400))
        })
        .expect("two blue answers it");
    game.apply(PlayerId::Two, counter).expect("it is cast");
    settle(&mut game);

    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::EPHEMERATE),
        "buried rather than exiled: rebound never happened",
    );
    assert!(
        game.players[0].exile.is_empty(),
        "nothing of it is waiting in exile",
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == blinked),
        "and the creature was never blinked: it is the object it always was",
    );

    to_the_rebound_offer(&mut game);
    assert!(
        game.pending_decisions.is_empty(),
        "and no upkeep offers it back",
    );
}

/// "Casting the card again due to rebound is optional. If you choose not to
/// cast it, the card will stay exiled. You won't get another chance to cast
/// it on a future turn."
#[test]
fn declining_the_rebound_strands_it_in_exile_for_good() {
    let (mut game, spell, blinked) = staged();
    cast(&mut game, spell, blinked);
    to_the_rebound_offer(&mut game);

    let decision = game
        .pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("the offer is waiting");
    // The offer carries the refusal as its one option; taking it is how the
    // permission is given back.
    let decline = decision
        .options
        .iter()
        .find(|option| option.label == "Decline")
        .expect("declining is one of the answers")
        .id;
    game.apply(
        decision.player,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![decline],
        },
    )
    .expect("declining is a legal answer");
    settle(&mut game);

    assert!(
        game.players[0]
            .exile
            .iter()
            .any(|card| card.definition == cards::EPHEMERATE),
        "the card it declined stays in exile",
    );
    assert!(
        game.players[0].graveyard.is_empty(),
        "and never reaches a graveyard",
    );

    // A whole turn of theirs and back round to another upkeep of ours.
    to_the_rebound_offer(&mut game);
    assert!(
        game.pending_decisions.is_empty(),
        "and it is not offered again on a later turn",
    );
}

/// The other half of the same ruling: "Auras attached to the exiled creature
/// will be put into their owners' graveyards." An Aura has nowhere to go
/// when its host stops existing, where an Equipment simply comes loose --
/// which is why blinking your own creature answers their Pacifism for good
/// and costs you your own Aura all the same.
#[test]
fn an_aura_on_what_it_blinks_goes_to_the_graveyard() {
    let (mut game, spell, _snapcaster) = staged();
    let bears = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    settle(&mut game);

    // Their Aura, already on it: Pacifism is sorcery-speed and this is not
    // their turn, so it is attached rather than cast.
    let mut pacifism = creature(94_500, cards::PACIFISM, PlayerId::Two);
    pacifism.attached_to = Some(bears);
    let pacifism_id = pacifism.card.id;
    game.battlefield.push(pacifism);
    game.check_state_based_actions();
    assert_eq!(
        game.attached_host(pacifism_id),
        Some(bears),
        "the Aura is on the Bears",
    );

    game.priority = PlayerId::One;
    cast(&mut game, spell, bears);

    let returned = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == ObjectKind::Card(cards::GRIZZLY_BEARS))
        .expect("the Bears came back");
    assert_ne!(returned.card.id, bears, "a new object came back");
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::PACIFISM),
        "and the Aura had nowhere to be",
    );
    assert!(
        game.players[1]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::PACIFISM),
        "so it went to its owner's graveyard",
    );
}
