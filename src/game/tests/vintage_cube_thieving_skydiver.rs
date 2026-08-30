//! Thieving Skydiver: a two-mana flier, or two plus X and the best artifact
//! on the other side of the table comes with him.

use super::*;

/// Player One holding the Skydiver with `mana` blue mana available.
fn staged(mana: u16) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let skydiver = game
        .build_zone(PlayerId::One, &[cards::THIEVING_SKYDIVER])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = skydiver.id;
    game.players[0].hand.push(skydiver);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, mana);
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
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

/// The casts of the Skydiver on offer, as (X, whether it is kicked).
fn casts(game: &Game, card: GameObjectId) -> Vec<(u16, bool)> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell {
                card: id, choices, ..
            } if id == card => Some((choices.x(), choices.costs().alternative().is_some())),
            _ => None,
        })
        .collect()
}

fn cast_kicked_for(game: &mut Game, card: GameObjectId, wanted: u16, target: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell {
                card: id, choices, ..
            } => *id == card && choices.x() == wanted,
            _ => false,
        })
        .unwrap_or_else(|| panic!("a kicked cast for X={wanted} is on offer"));
    game.apply(PlayerId::One, action).expect("it is cast");
    // The arrival trigger names the artifact.
    for _ in 0..16 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options: Vec<_> = decision
                .options
                .iter()
                .filter(|option| option.card.is_some_and(|(object, _)| object == target))
                .map(|option| option.id)
                .take(1)
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
    settle(game);
}

/// "X can't be 0": the kicked cast is never offered for nothing.
#[test]
fn the_kicker_is_never_free() {
    let (game, skydiver) = staged(4);

    let offered = casts(&game, skydiver);
    assert!(
        offered.contains(&(0, false)),
        "the plain two-mana cast is there: {offered:?}",
    );
    assert!(
        !offered.contains(&(0, true)),
        "and no kicked cast for X of nothing: {offered:?}",
    );
    assert!(
        offered.iter().any(|(x, kicked)| *kicked && *x == 1),
        "the smallest kick is one: {offered:?}",
    );
}

/// Kicked for one, he takes an artifact worth one.
#[test]
fn he_steals_an_artifact_worth_x() {
    let (mut game, skydiver) = staged(3);
    let key = game
        .put_onto_battlefield(PlayerId::Two, cards::MANIFOLD_KEY)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    cast_kicked_for(&mut game, skydiver, 1, key);

    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == key)
            .expect("it is still on the battlefield")
            .controller,
        PlayerId::One,
        "the artifact changed hands",
    );
}

/// An artifact that costs more than X is not a legal target, so the trigger
/// finds nothing and the artifact stays put.
#[test]
fn an_artifact_worth_more_than_x_is_safe() {
    let (mut game, skydiver) = staged(3);
    let greaves = game
        .put_onto_battlefield(PlayerId::Two, cards::LIGHTNING_GREAVES)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    let kicked_for_one = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if *card == skydiver && choices.x() == 1)
        })
        .expect("a kick of one is affordable");
    game.apply(PlayerId::One, kicked_for_one)
        .expect("it is cast");
    settle(&mut game);

    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == greaves)
            .expect("still there")
            .controller,
        PlayerId::Two,
        "a two-mana artifact is out of reach of a one-mana kick",
    );
}

/// An Equipment arrives already attached to him.
#[test]
fn a_stolen_equipment_attaches_itself() {
    let (mut game, skydiver) = staged(4);
    let greaves = game
        .put_onto_battlefield(PlayerId::Two, cards::LIGHTNING_GREAVES)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    cast_kicked_for(&mut game, skydiver, 2, greaves);

    let body = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::THIEVING_SKYDIVER)
        .expect("he resolved");
    assert_eq!(
        game.attached_host(greaves),
        Some(body.card.id),
        "the Equipment came down on him",
    );
    assert!(
        game.permanent_has_executable_keyword(body, KeywordAbility::Haste),
        "and what it grants is his: a stolen Equipment equips nobody else",
    );
}

/// "If it was kicked": cast for his printed two he is a 2/1 flier and
/// nothing else, and their artifact stays where it is.
#[test]
fn an_unkicked_skydiver_steals_nothing() {
    let (mut game, skydiver) = staged(2);
    let key = game
        .put_onto_battlefield(PlayerId::Two, cards::MANIFOLD_KEY)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    let plain = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell {
                card: id, choices, ..
            } => *id == skydiver && choices.x() == 0,
            _ => false,
        })
        .expect("two mana casts him without the kicker");
    game.apply(PlayerId::One, plain).expect("it is cast");
    settle(&mut game);
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::THIEVING_SKYDIVER),
        "he arrived",
    );
    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == key)
            .expect("their Key is untouched")
            .controller,
        PlayerId::Two,
        "and the trigger did nothing, because it was never kicked",
    );
}

/// "The control-change effect lasts indefinitely ... it doesn't expire if
/// Thieving Skydiver leaves the battlefield."
#[test]
fn what_he_took_stays_taken_after_he_dies() {
    let (mut game, skydiver) = staged(3);
    let key = game
        .put_onto_battlefield(PlayerId::Two, cards::MANIFOLD_KEY)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;
    cast_kicked_for(&mut game, skydiver, 1, key);

    let thief = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::THIEVING_SKYDIVER)
        .expect("he is on the battlefield")
        .card
        .id;
    game.move_permanents_to_graveyard(&[thief]);
    settle(&mut game);
    game.cleanup();
    game.check_state_based_actions();

    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == key)
            .expect("the Key is still on the battlefield")
            .controller,
        PlayerId::One,
        "it does not go home when the thief does",
    );
}

/// "Thieving Skydiver's ability can target an artifact you already control.
/// You'll attach it to Thieving Skydiver if it's an Equipment." Stealing from
/// yourself gains you nothing, but it does move the Boots.
#[test]
fn he_can_take_an_equipment_off_your_own_creature() {
    let (mut game, skydiver) = staged(4);
    let bear = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    let greaves = game
        .put_onto_battlefield(PlayerId::One, cards::LIGHTNING_GREAVES)
        .expect("cataloged");
    assert!(game.try_attach(greaves, bear), "they start on the Bears");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    cast_kicked_for(&mut game, skydiver, 2, greaves);

    let body = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::THIEVING_SKYDIVER)
        .expect("he resolved");
    assert_eq!(
        game.attached_host(greaves),
        Some(body.card.id),
        "his own ability pulls them off the Bears and onto him",
    );
}

/// "If you put a permanent with a kicker ability onto the battlefield without
/// casting it, you can't kick it." Arriving without a cast is not arriving
/// kicked, so the trigger asks for nothing.
#[test]
fn arriving_without_a_cast_is_never_kicked() {
    let (mut game, _skydiver) = staged(0);
    let key = game
        .put_onto_battlefield(PlayerId::Two, cards::MANIFOLD_KEY)
        .expect("cataloged");
    game.put_onto_battlefield(PlayerId::One, cards::THIEVING_SKYDIVER)
        .expect("cataloged");
    drain_pending(&mut game);
    settle(&mut game);

    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == key)
            .expect("their Key is still there")
            .controller,
        PlayerId::Two,
        "no cast means no kick, however much mana was lying around",
    );
}

/// "If the Equipment can't be attached to Thieving Skydiver, most likely
/// because Thieving Skydiver has left the battlefield before its triggered
/// ability resolves, the Equipment remains attached to whatever it's
/// currently attached to." Killing the thief in response saves the Bears
/// their Boots, but not their owner: the theft half still happens.
#[test]
fn killing_him_in_response_leaves_the_equipment_where_it_was() {
    let (mut game, skydiver) = staged(4);
    let bear = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    let greaves = game
        .put_onto_battlefield(PlayerId::Two, cards::LIGHTNING_GREAVES)
        .expect("cataloged");
    assert!(game.try_attach(greaves, bear), "they start on their Bears");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell {
                card: id, choices, ..
            } => *id == skydiver && choices.x() == 2,
            _ => false,
        })
        .expect("a kicked cast for X=2 is on offer");
    game.apply(PlayerId::One, action).expect("it is cast");

    // Let him resolve and name the Boots, then stop with the trigger waiting.
    for _ in 0..16 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options: Vec<_> = decision
                .options
                .iter()
                .filter(|option| option.card.is_some_and(|(object, _)| object == greaves))
                .map(|option| option.id)
                .take(1)
                .collect();
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the Boots are a legal target");
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }

    let thief = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::THIEVING_SKYDIVER)
        .expect("he is on the battlefield with his trigger still waiting")
        .card
        .id;
    game.move_permanents_to_graveyard(&[thief]);
    settle(&mut game);
    game.check_state_based_actions();

    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == greaves)
            .expect("the Boots are still on the battlefield")
            .controller,
        PlayerId::One,
        "the theft resolves without him",
    );
    assert_eq!(
        game.attached_host(greaves),
        Some(bear),
        "but with nobody to attach them to, they stay on the Bears",
    );
}

/// "The spell's mana value remains unchanged, no matter what the total cost
/// to cast it was." A Skydiver kicked for three cost five mana and is still
/// a two-drop on the battlefield.
#[test]
fn kicking_him_does_not_change_what_he_costs() {
    let (mut game, skydiver) = staged(5);
    let key = game
        .put_onto_battlefield(PlayerId::Two, cards::MANIFOLD_KEY)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    cast_kicked_for(&mut game, skydiver, 3, key);

    let body = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::THIEVING_SKYDIVER)
        .expect("he arrived");
    assert_eq!(
        game.permanent_mana_value(body),
        2,
        "the kicker is a cost increase, not a bigger card",
    );
}

/// "If a card or token enters as a copy of a permanent, the new permanent
/// isn't kicked, even if the original was." A Metamorph copying a kicked
/// Skydiver arrives having been cast as a Metamorph, so its own arrival
/// steals nothing.
#[test]
fn a_copy_of_him_was_never_kicked_itself() {
    let (mut game, skydiver) = staged(5);
    let key = game
        .put_onto_battlefield(PlayerId::Two, cards::MANIFOLD_KEY)
        .expect("cataloged");
    let second_key = game
        .put_onto_battlefield(PlayerId::Two, cards::MANIFOLD_KEY)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;
    cast_kicked_for(&mut game, skydiver, 1, key);
    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == key)
            .expect("the first Key is still there")
            .controller,
        PlayerId::One,
        "the kicked Skydiver took one",
    );

    // A Metamorph copying him is cast as a Metamorph, kicker or no kicker.
    let metamorph = game
        .build_zone(PlayerId::One, &[cards::PHYREXIAN_METAMORPH])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let metamorph_id = metamorph.id;
    game.players[0].hand.push(metamorph);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 5);
    let body = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::THIEVING_SKYDIVER)
        .expect("he is on the battlefield")
        .card
        .id;
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == metamorph_id))
        .expect("five mana casts it");
    game.apply(PlayerId::One, action).expect("it is cast");
    for _ in 0..12 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .find(|option| option.card.is_some_and(|(object, _)| object == body))
                .map_or_else(|| vec![decision.options[0].id], |option| vec![option.id]);
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the answer is legal");
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
    settle(&mut game);

    assert_eq!(
        game.battlefield
            .iter()
            .filter(|permanent| permanent.card.definition == cards::THIEVING_SKYDIVER)
            .count()
            + game
                .battlefield
                .iter()
                .filter(|permanent| permanent.card.definition == cards::PHYREXIAN_METAMORPH)
                .count(),
        2,
        "there are two Skydivers on the battlefield now",
    );
    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == second_key)
            .expect("the second Key is still there")
            .controller,
        PlayerId::Two,
        "and the copy, never having been kicked, took nothing",
    );
}
