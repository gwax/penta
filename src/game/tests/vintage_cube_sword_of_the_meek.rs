//! Sword of the Meek: an Equipment that climbs out of the graveyard and
//! straps itself to whatever 1/1 just showed up.

use super::*;

/// Player One with a Sword in the graveyard and nothing on the battlefield.
fn staged() -> Game {
    let mut game = ready_game();
    game.battlefield.clear();
    let sword = game
        .build_zone(PlayerId::One, &[cards::SWORD_OF_THE_MEEK])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    game.players[0].graveyard.push(sword);
    game
}

/// Answers every pending decision, saying yes to any "may", then resolves
/// whatever is left on the stack.
fn settle(game: &mut Game, accept: bool) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let wanted = if accept {
                decision
                    .options
                    .iter()
                    .find(|option| option.label != "Decline")
            } else {
                decision
                    .options
                    .iter()
                    .find(|option| option.label == "Decline")
            };
            let options = wanted
                .or_else(|| decision.options.first())
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
            return;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            return;
        }
    }
}

fn sword_on_battlefield(game: &Game) -> Option<&Permanent> {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::SWORD_OF_THE_MEEK)
}

fn power_toughness(game: &Game, id: GameObjectId) -> (Option<i16>, Option<i16>) {
    let Some(permanent) = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
    else {
        return (None, None);
    };
    (game.power(permanent), game.toughness(permanent))
}

/// A 1/1 arriving brings the Sword back and wears it.
#[test]
fn a_one_one_pulls_the_sword_out_of_the_graveyard() {
    let mut game = staged();
    let servant = game
        .put_onto_battlefield(PlayerId::One, cards::MERFOLK_OF_THE_PEARL_TRIDENT)
        .expect("cataloged");
    settle(&mut game, true);

    let sword = sword_on_battlefield(&game).expect("the Sword came back");
    assert_eq!(
        sword.attached_to,
        Some(servant),
        "and attached itself to what brought it",
    );
    assert!(
        game.players[0].graveyard.is_empty(),
        "it left the graveyard behind",
    );
    assert_eq!(
        power_toughness(&game, servant),
        (Some(2), Some(3)),
        "a 1/1 wearing +1/+2",
    );
}

/// Declining leaves it where it was.
#[test]
fn declining_leaves_the_sword_in_the_graveyard() {
    let mut game = staged();
    game.put_onto_battlefield(PlayerId::One, cards::MERFOLK_OF_THE_PEARL_TRIDENT)
        .expect("cataloged");
    settle(&mut game, false);

    assert!(sword_on_battlefield(&game).is_none());
    assert_eq!(game.players[0].graveyard.len(), 1);
}

/// A creature that is not a 1/1 does not wake it up.
#[test]
fn a_bigger_creature_does_not_wake_it() {
    let mut game = staged();
    game.put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    settle(&mut game, true);

    assert!(sword_on_battlefield(&game).is_none(), "a 2/2 is not a 1/1");
    assert_eq!(game.players[0].graveyard.len(), 1);
}

/// Nor does an opponent's 1/1: the clause says a creature you control.
#[test]
fn an_opponents_one_one_does_not_wake_it() {
    let mut game = staged();
    game.put_onto_battlefield(PlayerId::Two, cards::MERFOLK_OF_THE_PEARL_TRIDENT)
        .expect("cataloged");
    settle(&mut game, true);

    assert!(sword_on_battlefield(&game).is_none());
    assert_eq!(game.players[0].graveyard.len(), 1);
}

/// "Any creature can be equipped with Sword of the Meek, not just 1/1
/// creatures." The trigger is what asks about 1/1s; equip asks nothing.
#[test]
fn equip_names_any_creature_of_yours() {
    let mut game = ready_game();
    game.battlefield.clear();
    let sword_id = game
        .put_onto_battlefield(PlayerId::One, cards::SWORD_OF_THE_MEEK)
        .expect("cataloged");
    let bears = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);

    let equip = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } => {
                *source == sword_id
                    && targets
                        .iter()
                        .flat_map(crate::casting::TargetSelection::targets)
                        .any(|target| *target == Target::Permanent(bears))
            }
            _ => false,
        })
        .expect("a 2/2 is a creature you control");
    game.apply(PlayerId::One, equip).expect("it equips");
    drain_pending(&mut game);

    assert_eq!(
        power_toughness(&game, bears),
        (Some(3), Some(4)),
        "the +1/+2 does not care what it is worn by",
    );
}

/// "It triggers only if it's in your graveyard immediately after the 1/1
/// enters." A Sword already on the battlefield is not a Sword to return.
#[test]
fn a_sword_already_out_does_not_trigger() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].graveyard.clear();
    game.put_onto_battlefield(PlayerId::One, cards::SWORD_OF_THE_MEEK)
        .expect("cataloged");
    drain_pending(&mut game);

    game.put_onto_battlefield(PlayerId::One, cards::MERFOLK_OF_THE_PEARL_TRIDENT)
        .expect("cataloged");
    game.finish_rules_procedure();

    assert!(
        game.pending_decisions.is_empty() && game.pending_triggers.is_empty(),
        "there was nothing in the graveyard to ask about",
    );
}

/// "If the Sword can't be attached to the creature that caused its ability
/// to trigger, most likely because that creature has left the battlefield,
/// it returns to the battlefield and remains unattached."
#[test]
fn a_dead_host_leaves_the_sword_bare() {
    let mut game = staged();
    let servant = game
        .put_onto_battlefield(PlayerId::One, cards::MERFOLK_OF_THE_PEARL_TRIDENT)
        .expect("cataloged");
    game.finish_rules_procedure();
    game.move_permanents_to_graveyard(&[servant]);
    settle(&mut game, true);

    let sword = sword_on_battlefield(&game).expect("it still came back");
    assert_eq!(
        sword.attached_to, None,
        "with nothing left to attach itself to",
    );
}

/// "Once the last ability has triggered, changing the power or toughness of
/// the creature won't stop you from returning Sword of the Meek and
/// attaching it to the creature." A Giant Growth in response makes a 4/4 of
/// the Merfolk, and the Sword comes back to it regardless.
#[test]
fn pumping_the_host_after_the_trigger_changes_nothing() {
    let mut game = staged();
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    let growth = card(84_500, cards::GIANT_GROWTH, PlayerId::One);
    let growth_id = growth.id;
    game.players[0].hand.push(growth);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 1);
    let servant = game
        .put_onto_battlefield(PlayerId::One, cards::MERFOLK_OF_THE_PEARL_TRIDENT)
        .expect("cataloged");

    // The trigger is waiting; the Merfolk stops being a 1/1 under it.
    for _ in 0..4 {
        if !game.stack.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    assert_eq!(game.stack.len(), 1, "the Sword's trigger is on the stack");
    game.priority = PlayerId::One;
    game.apply(
        PlayerId::One,
        cast_action(growth_id, vec![Target::Permanent(servant)], Vec::new(), 0),
    )
    .expect("one green mana pumps it");
    settle(&mut game, true);

    assert_eq!(
        power_toughness(&game, servant),
        (Some(5), Some(6)),
        "1/1 plus three plus the Sword's own +1/+2",
    );
    let sword = sword_on_battlefield(&game).expect("the Sword still came back");
    assert_eq!(
        sword.attached_to,
        Some(servant),
        "and it is wearing the creature that is no longer a 1/1",
    );
}

/// Its ruling: "if a creature is entering the battlefield under your
/// control, consider static abilities to determine whether its power and
/// toughness are both 1." A Crusade already out makes the arriving Mother a
/// 2/2, and a 2/2 is not what the Sword is waiting for.
#[test]
fn a_static_pump_is_read_as_the_creature_arrives() {
    for crusade in [false, true] {
        let mut game = staged();
        if crusade {
            game.put_onto_battlefield(PlayerId::One, cards::CRUSADE)
                .expect("cataloged");
            drain_pending(&mut game);
        }
        let mother = game
            .put_onto_battlefield(PlayerId::One, cards::MOTHER_OF_RUNES)
            .expect("cataloged");
        // Read before the Sword can dress her, which is where the trigger
        // read it too.
        assert_eq!(
            power_toughness(&game, mother),
            if crusade {
                (Some(2), Some(2))
            } else {
                (Some(1), Some(1))
            },
            "the Mother arrived at the size the board made her, crusade={crusade}",
        );
        settle(&mut game, true);

        assert_eq!(
            sword_on_battlefield(&game).is_none(),
            crusade,
            "and the Sword read that size rather than the printed one",
        );
    }
}

/// The same rule read the other way, which is the half a Sword deck cares
/// about: an Engineered Plague naming Bears makes the arriving Grizzly Bears
/// a 1/1, and a 1/1 is exactly what the Sword is waiting for.
#[test]
fn a_static_shrink_makes_a_two_two_into_what_it_wants() {
    let mut game = staged();
    game.put_onto_battlefield(PlayerId::One, cards::ENGINEERED_PLAGUE)
        .expect("cataloged");
    let choice = game
        .observe(PlayerId::One)
        .decision
        .expect("the Plague names a creature type");
    let bear = choice
        .options
        .iter()
        .find(|option| option.label == "Bear")
        .expect("Bear is offered")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: choice.id,
            options: vec![bear],
        },
    )
    .expect("naming it is legal");
    drain_pending(&mut game);

    let bears = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    assert_eq!(
        power_toughness(&game, bears),
        (Some(1), Some(1)),
        "the Plague shrank it on the way in",
    );
    settle(&mut game, true);

    let sword = sword_on_battlefield(&game).expect("a 1/1 is a 1/1 however it got there");
    assert_eq!(sword.attached_to, Some(bears), "and it dressed the Bears");
}

/// The shape the card is actually played in: more than one Sword waiting.
/// Each is its own trigger, each is offered separately, and a creature may
/// wear both -- one Merfolk arriving empties a graveyard of two Swords and
/// stands there as a 3/5.
#[test]
fn two_swords_in_the_graveyard_both_come_back_to_the_same_creature() {
    let mut game = staged();
    let second = game
        .build_zone(PlayerId::One, &[cards::SWORD_OF_THE_MEEK])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    game.players[0].graveyard.push(second);
    assert_eq!(game.players[0].graveyard.len(), 2, "two of them waiting");

    let servant = game
        .put_onto_battlefield(PlayerId::One, cards::MERFOLK_OF_THE_PEARL_TRIDENT)
        .expect("cataloged");

    // Two triggers means an ordering question before the two offers, and
    // that one wants both answers rather than one.
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options: Vec<_> = if decision.minimum > 1 {
                decision
                    .options
                    .iter()
                    .map(|option| option.id)
                    .take(decision.minimum)
                    .collect()
            } else {
                decision
                    .options
                    .iter()
                    .find(|option| option.label != "Decline")
                    .or_else(|| decision.options.first())
                    .map(|option| vec![option.id])
                    .unwrap_or_default()
            };
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

    let swords: Vec<_> = game
        .battlefield
        .iter()
        .filter(|permanent| permanent.card.definition == cards::SWORD_OF_THE_MEEK)
        .collect();
    assert_eq!(swords.len(), 2, "both Swords came back");
    assert!(
        swords
            .iter()
            .all(|sword| sword.attached_to == Some(servant)),
        "and both went onto the creature that called them",
    );
    assert!(
        game.players[0].graveyard.is_empty(),
        "with nothing left in the graveyard",
    );
    assert_eq!(
        power_toughness(&game, servant),
        (Some(3), Some(5)),
        "a 1/1 wearing two of them",
    );
}
