//! Rancor: two power, trample, and a card that will not stay dead.

use super::*;

/// Rancor on a creature: bigger, trampling, and still there.
#[test]
fn rancor_grants_two_power_and_trample() {
    let mut game = ready_game();
    game.battlefield.clear();
    let bears = creature(54_000, cards::GRIZZLY_BEARS, PlayerId::One);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    let rancor = card(54_001, cards::RANCOR, PlayerId::One);
    let rancor_id = rancor.id;
    game.players[PlayerId::One.index()].hand.push(rancor);
    game.players[PlayerId::One.index()].mana_pool.green = 1;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
            if *card == rancor_id
                && choices.targets().iter().any(|selection| {
                    selection.targets().contains(&Target::Permanent(bears_id))
                }))
        })
        .expect("Rancor targets a creature");
    game.apply(PlayerId::One, action).expect("it is cast");
    drain_pending(&mut game);

    let bears = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == bears_id)
        .expect("the creature is still there");
    assert_eq!(
        (game.power(bears), game.toughness(bears)),
        (Some(4), Some(2))
    );
    assert!(game.has_trample(bears));
}

/// The clause the card is remembered for. Whichever half of the pair is
/// answered, the Aura reaches the graveyard, and it is the graveyard object
/// -- a different object from the permanent that just left -- that comes back
/// to hand.
#[test]
fn rancor_returns_itself_to_hand_from_the_graveyard() {
    for kill_the_creature in [false, true] {
        let mut game = ready_game();
        game.battlefield.clear();
        game.players[PlayerId::One.index()].hand.clear();
        let bears = creature(54_100, cards::GRIZZLY_BEARS, PlayerId::One);
        let bears_id = bears.card.id;
        game.battlefield.push(bears);
        let rancor = card(54_101, cards::RANCOR, PlayerId::One);
        let rancor_id = rancor.id;
        game.players[PlayerId::One.index()].hand.push(rancor);
        game.players[PlayerId::One.index()].mana_pool.green = 1;

        let action = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| {
                matches!(action, Action::CastSpell { card, choices, .. }
                if *card == rancor_id
                    && choices.targets().iter().any(|selection| {
                        selection.targets().contains(&Target::Permanent(bears_id))
                    }))
            })
            .expect("Rancor targets a creature");
        game.apply(PlayerId::One, action).expect("it is cast");
        drain_pending(&mut game);

        // The Aura on the battlefield is a new object; the hand card's id is
        // not the permanent's.
        let aura = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.definition == cards::RANCOR)
            .expect("the Aura attached")
            .card
            .id;
        let doomed = if kill_the_creature { bears_id } else { aura };
        game.move_permanents_to_graveyard(&[doomed]);
        // Killing the creature leaves the Aura enchanting nothing; it takes
        // state-based actions to notice and send it after its host.
        game.check_state_based_actions();
        drain_pending(&mut game);

        assert!(
            game.players[PlayerId::One.index()]
                .hand
                .iter()
                .any(|card| card.definition == cards::RANCOR),
            "Rancor comes back whether the creature or the Aura was answered \
             (creature killed: {kill_the_creature})",
        );
        assert!(
            game.players[PlayerId::One.index()]
                .graveyard
                .iter()
                .all(|card| card.definition != cards::RANCOR),
            "and it does not stay in the graveyard as well",
        );
    }
}

/// "If the creature this Aura would enchant is an illegal target by the
/// time Rancor tries to resolve, the Aura spell doesn't resolve. It won't
/// enter the battlefield, so it won't be put into a graveyard from the
/// battlefield and its ability won't trigger." Which is the one way to be
/// rid of a Rancor for good: answer the creature while the Aura is still a
/// spell.
#[test]
fn a_rancor_that_never_resolves_stays_in_the_graveyard() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::Two.index()].hand.clear();
    let bears = creature(54_200, cards::GRIZZLY_BEARS, PlayerId::One);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    let rancor = card(54_201, cards::RANCOR, PlayerId::One);
    let rancor_id = rancor.id;
    game.players[PlayerId::One.index()].hand.push(rancor);
    game.players[PlayerId::One.index()].mana_pool.green = 1;
    game.players[PlayerId::Two.index()].hand.push(card(
        54_202,
        cards::LIGHTNING_BOLT,
        PlayerId::Two,
    ));
    game.players[PlayerId::Two.index()].mana_pool.red = 1;

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
            if *card == rancor_id
                && choices.targets().iter().any(|selection| {
                    selection.targets().contains(&Target::Permanent(bears_id))
                }))
        })
        .expect("Rancor targets a creature");
    game.apply(PlayerId::One, cast).expect("it is cast");

    // They answer the bear while the Aura is still on the stack.
    game.priority = PlayerId::Two;
    let bolt = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
            if *card == CardInstanceId(54_202)
                && choices.targets().iter().any(|selection| {
                    selection.targets().contains(&Target::Permanent(bears_id))
                }))
        })
        .expect("one red answers the bear");
    game.apply(PlayerId::Two, bolt).expect("it is cast");
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == bears_id),
        "the bear was answered first",
    );
    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::RANCOR),
        "and the Aura, having never resolved, is in the graveyard",
    );
    assert!(
        game.players[PlayerId::One.index()]
            .hand
            .iter()
            .all(|card| card.definition != cards::RANCOR),
        "with nothing to return it: it was never on the battlefield to leave it",
    );
}
