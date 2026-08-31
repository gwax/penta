//! Crop Rotation: one green mana and a land you already have for any land in
//! the deck, at instant speed.

use super::*;

/// Player One holding the Rotation with a green mana up, `lands` on the
/// battlefield, and a Gaea's Cradle waiting in the library.
fn staged(lands: usize) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    game.players[0]
        .library
        .push(card(126_000, cards::GAEAS_CRADLE, PlayerId::One));
    let mut ids = Vec::new();
    for index in 0..lands {
        let land = creature(
            126_100 + u32::try_from(index).expect("a few lands"),
            cards::FOREST,
            PlayerId::One,
        );
        ids.push(land.card.id);
        game.battlefield.push(land);
    }
    let rotation = card(126_200, cards::CROP_ROTATION, PlayerId::One);
    let rotation_id = rotation.id;
    game.players[0].hand.push(rotation);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 1);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, rotation_id, ids)
}

/// Every way the Rotation is castable right now, with what each would give
/// up for it.
fn casts(game: &Game, rotation: GameObjectId) -> Vec<Vec<GameObjectId>> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell {
                card, sacrifices, ..
            } if card == rotation => Some(sacrifices),
            _ => None,
        })
        .collect()
}

/// "You can't sacrifice additional lands": every offer gives up exactly one,
/// and there is one offer per land.
#[test]
fn it_gives_up_one_land_and_only_one() {
    let (game, rotation, lands) = staged(3);

    let offers = casts(&game, rotation);
    assert_eq!(offers.len(), 3, "one way per land: {offers:?}");
    for offer in &offers {
        assert_eq!(offer.len(), 1, "one land each, never two: {offer:?}");
        assert!(lands.contains(&offer[0]));
    }
}

/// It is an instant, so the land it gives up can be one that was about to
/// be answered anyway -- on the other player's turn, with their spell still
/// on the stack.
#[test]
fn it_answers_on_their_turn_with_the_stack_full() {
    let (mut game, rotation, lands) = staged(1);
    game.active_player = PlayerId::Two;
    game.step = Step::PrecombatMain;
    let bolt = card(126_300, cards::LIGHTNING_BOLT, PlayerId::Two);
    let bolt_id = bolt.id;
    game.players[1].hand.push(bolt);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Red, 1);
    game.priority = PlayerId::Two;
    game.apply(
        PlayerId::Two,
        cast_action(bolt_id, vec![Target::Player(PlayerId::One)], Vec::new(), 0),
    )
    .expect("their Bolt is cast");
    game.priority = PlayerId::One;

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, sacrifices, .. }
                if *card == rotation && sacrifices.contains(&lands[0]))
        })
        .expect("an instant is an instant");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);
    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::GAEAS_CRADLE),
        "the Cradle arrived while their Bolt was still on the stack",
    );
    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != lands[0]),
        "and the Forest went as the cost",
    );
}

/// Casts it, giving up `land`, and answers the search by taking `wanted`
/// when it is offered.
fn rotate(game: &mut Game, rotation: GameObjectId, land: GameObjectId, wanted: bool) {
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, sacrifices, .. }
                if *card == rotation && sacrifices.contains(&land))
        })
        .expect("that land pays for it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    for _ in 0..12 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = if wanted {
                decision
                    .options
                    .iter()
                    .map(|option| option.id)
                    .take(1)
                    .collect()
            } else {
                Vec::new()
            };
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
    drain_pending(game);
}

/// The sacrifice is a cost rather than part of what resolves: countering the
/// Rotation takes the search away and leaves the land spent.
#[test]
fn a_countered_rotation_still_costs_the_land() {
    let (mut game, rotation, lands) = staged(1);
    let counter = card(126_400, cards::COUNTERSPELL, PlayerId::Two);
    let counter_id = counter.id;
    game.players[1].hand.push(counter);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Blue, 2);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, sacrifices, .. }
                if *card == rotation && sacrifices.contains(&lands[0]))
        })
        .expect("the Forest pays for it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != lands[0]),
        "the land went as the spell was cast, not as it resolved",
    );

    let spell = game
        .stack
        .iter()
        .next()
        .map(|object| object.id)
        .expect("the Rotation is on the stack");
    game.apply(PlayerId::One, Action::PassPriority)
        .expect("priority passes");
    game.apply(
        PlayerId::Two,
        cast_action(counter_id, vec![Target::Spell(spell)], Vec::new(), 0),
    )
    .expect("they answer it");
    drain_pending(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::GAEAS_CRADLE),
        "nothing was found",
    );
    assert!(
        game.players[0].library.len() == 1,
        "the Cradle is still in the library",
    );
}

/// The search may find nothing: the land is spent either way, and what is
/// left is a board one land smaller.
#[test]
fn declining_the_search_still_spends_the_land() {
    let (mut game, rotation, lands) = staged(2);

    rotate(&mut game, rotation, lands[0], false);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::GAEAS_CRADLE),
        "nothing was taken",
    );
    assert_eq!(
        game.battlefield
            .iter()
            .filter(|permanent| permanent.card.definition == cards::FOREST)
            .count(),
        1,
        "and the land it cost is gone",
    );
}

/// "Put that card onto the battlefield" says nothing about tapped, so what
/// arrives is what the land itself says: a Cradle untapped, a triome tapped.
#[test]
fn what_it_finds_arrives_as_that_land_says() {
    for (definition, tapped) in [(cards::GAEAS_CRADLE, false), (cards::KETRIA_TRIOME, true)] {
        let (mut game, rotation, lands) = staged(1);
        game.players[0].library.clear();
        game.players[0]
            .library
            .push(card(126_500, definition, PlayerId::One));

        rotate(&mut game, rotation, lands[0], true);

        let found = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.definition == definition)
            .unwrap_or_else(|| panic!("{definition:?} was found"));
        assert_eq!(
            found.tapped, tapped,
            "{definition:?} arrives as its own clause says",
        );
    }
}
