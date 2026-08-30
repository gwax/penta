//! Jace, Vryn's Prodigy: a looter that turns into a planeswalker the moment
//! the graveyard is deep enough.

use super::*;

/// Jace on the battlefield since last turn, with `buried` in the graveyard
/// and a library to loot from.
fn staged(buried: usize) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    game.players[0].library.clear();
    for index in 0..6 {
        game.players[0]
            .library
            .push(card(68_000 + index, cards::ISLAND, PlayerId::One));
    }
    for index in 0..buried {
        let id = 68_100 + u32::try_from(index).expect("a handful of cards");
        game.players[0]
            .graveyard
            .push(card(id, cards::LIGHTNING_BOLT, PlayerId::One));
    }
    let jace = game
        .put_onto_battlefield(PlayerId::One, cards::JACE_VRYN_S_PRODIGY)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turns_started = [6, 6];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, jace)
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

fn loot(game: &mut Game, jace: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == jace))
        .expect("the loot is offered");
    game.apply(PlayerId::One, action).expect("it activates");
    settle(game);
}

fn permanent_named<'a>(game: &'a Game, name: &str) -> Option<&'a Permanent> {
    game.battlefield
        .iter()
        .find(|permanent| game.effective_permanent_name(permanent).as_deref() == Some(name))
}

/// Three cards in the graveyard: the loot makes it four, which is not five.
#[test]
fn a_shallow_graveyard_leaves_him_a_creature() {
    let (mut game, jace) = staged(3);

    loot(&mut game, jace);

    assert_eq!(game.players[0].graveyard.len(), 4, "the discard went in");
    assert!(
        permanent_named(&game, "Jace, Vryn's Prodigy").is_some(),
        "and he is still the Wizard",
    );
}

/// Four already there and the discard makes five, so he flips on the same
/// activation that fills it.
#[test]
fn the_discard_itself_can_be_the_fifth_card() {
    let (mut game, jace) = staged(4);

    loot(&mut game, jace);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == jace),
        "the creature he was is gone",
    );
    let unbound = permanent_named(&game, "Jace, Telepath Unbound").expect("he came back");
    assert_eq!(
        unbound.counters(CounterKind::Loyalty),
        5,
        "with the loyalty the back face prints",
    );
    assert_eq!(unbound.controller, PlayerId::One);
}

/// The tap is a cost, so a Jace that arrived this turn has nothing to do
/// with it.
#[test]
fn a_fresh_jace_cannot_loot() {
    let (mut game, jace) = staged(4);
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == jace)
        .expect("he is there")
        .entered_controller_turn = game.turns_started[0];

    assert!(
        !game.legal_actions(PlayerId::One).iter().any(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == jace)
        ),
        "summoning sickness stops the tap",
    );
}

/// The back face's plus shrinks a creature, and until your next turn rather
/// than until end of turn.
#[test]
fn the_plus_one_shrinks_a_creature() {
    let (mut game, jace) = staged(4);
    let bears = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    loot(&mut game, jace);
    let unbound = permanent_named(&game, "Jace, Telepath Unbound")
        .expect("he flipped")
        .card
        .id;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    let plus = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { source, targets, .. }
                if *source == unbound && targets.iter().any(|selection| {
                    selection.targets().iter().any(|target| matches!(target, Target::Permanent(id) if *id == bears))
                }))
        })
        .expect("the plus can name their Bear");
    game.apply(PlayerId::One, plus).expect("it activates");
    settle(&mut game);

    let bear = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == bears)
        .expect("it is still there");
    assert_eq!(game.power(bear), Some(0), "-2/-0");
    assert_eq!(game.toughness(bear), Some(2), "and its toughness is left");

    game.cleanup();
    let bear = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == bears)
        .expect("it is still there");
    assert_eq!(
        game.power(bear),
        Some(0),
        "still shrunk after cleanup: the clause lasts until your next turn",
    );
}

/// The minus three hands a card in the graveyard back, for its own cost,
/// and exiles it afterwards.
#[test]
fn the_minus_three_buys_back_a_spell() {
    let (mut game, jace) = staged(4);
    loot(&mut game, jace);
    let unbound = permanent_named(&game, "Jace, Telepath Unbound")
        .expect("he flipped")
        .card
        .id;
    let bolt = game.players[0]
        .graveyard
        .iter()
        .find(|card| card.definition == cards::LIGHTNING_BOLT)
        .expect("a Bolt is in there")
        .id;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    let minus = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { source, targets, .. }
                if *source == unbound && targets.iter().any(|selection| {
                    selection.targets().iter().any(|target| matches!(target, Target::Card(id) if *id == bolt))
                }))
        })
        .expect("the minus can name the Bolt");
    game.apply(PlayerId::One, minus).expect("it activates");
    settle(&mut game);

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if *card == bolt
                    && choices
                        .iter_targets()
                        .any(|target| matches!(target, Target::Player(PlayerId::Two))))
        })
        .expect("the Bolt is castable from the graveyard, at them");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game);

    assert_eq!(game.players[1].life, 17, "the Bolt resolved");
    assert!(
        game.players[0]
            .exile
            .iter()
            .any(|card| card.definition == cards::LIGHTNING_BOLT),
        "and it was exiled rather than left in the graveyard",
    );
}

/// "Jace entering the battlefield while there are five or more cards in your
/// graveyard" is not what flips him: only the activation asks, and only
/// after its own discard.
#[test]
fn a_full_graveyard_does_not_flip_him_by_itself() {
    let (game, jace) = staged(6);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == jace),
        "he is still the creature he entered as",
    );
    assert!(
        permanent_named(&game, "Jace, Telepath Unbound").is_none(),
        "nothing has asked the question yet",
    );
    assert_eq!(
        game.players[0].graveyard.len(),
        6,
        "with a graveyard that would answer it",
    );
}

/// "You can activate one of the planeswalker's loyalty abilities the turn it
/// enters the battlefield." He flips in the middle of your main phase, and
/// the plus is there to be used at once.
#[test]
fn the_flipped_planeswalker_may_move_at_once() {
    let (mut game, jace) = staged(4);
    let bears = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    loot(&mut game, jace);
    let unbound = permanent_named(&game, "Jace, Telepath Unbound")
        .expect("he came back as the planeswalker")
        .card
        .id;

    let plus = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source,
                ability: AbilityOrigin::Printed { ability, .. },
                targets,
                ..
            } => {
                *source == unbound
                    && *ability == AbilityId(0)
                    && targets
                        .iter()
                        .any(|selection| selection.targets() == [Target::Permanent(bears)])
            }
            _ => false,
        })
        .expect("the plus is offered on the turn he arrived");
    game.apply(PlayerId::One, plus).expect("it activates");
    settle(&mut game);

    let shrunk = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == bears)
        .expect("the Bears are still there");
    assert_eq!(game.power(shrunk), Some(0), "two power off a 2/2");
    assert_eq!(
        permanent_named(&game, "Jace, Telepath Unbound")
            .expect("he is still there")
            .counters(CounterKind::Loyalty),
        6,
        "and the plus took him to six",
    );
}

/// "You can control two of this permanent, one front-face up and the other
/// back-face up, at the same time." The legend rule reads names, and the two
/// faces do not share one.
#[test]
fn a_flipped_jace_stands_beside_an_unflipped_one() {
    let (mut game, jace) = staged(4);
    loot(&mut game, jace);
    assert!(
        permanent_named(&game, "Jace, Telepath Unbound").is_some(),
        "the fifth card flipped him",
    );

    game.put_onto_battlefield(PlayerId::One, cards::JACE_VRYN_S_PRODIGY)
        .expect("cataloged");
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert!(
        permanent_named(&game, "Jace, Telepath Unbound").is_some(),
        "the planeswalker is still there",
    );
    assert!(
        permanent_named(&game, "Jace, Vryn's Prodigy").is_some(),
        "and so is the Wizard: two names, not one",
    );
}

/// "A double-faced permanent with its back face up has a mana value equal to
/// the mana value of its front face." The planeswalker face prints no cost
/// of its own, and two is what the Wizard cost.
#[test]
fn the_flipped_planeswalker_keeps_the_front_faces_mana_value() {
    let (mut game, jace) = staged(4);
    loot(&mut game, jace);

    let walker = permanent_named(&game, "Jace, Telepath Unbound").expect("he flipped");

    assert_eq!(
        game.permanent_mana_value(walker),
        2,
        "one and a blue is what he is worth on either face",
    );
}
