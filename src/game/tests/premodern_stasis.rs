//! The cards the Premodern Stasis list needed.

use super::*;

/// Forsaken City is a land that stays tapped unless you feed it, which is
/// what makes it playable only in a deck holding cards it will not cast.
#[test]
fn forsaken_city_stays_tapped_until_a_card_is_exiled_for_it() {
    let mut game = ready_game();
    let mut city = creature(10_000, cards::FORSAKEN_CITY, PlayerId::One);
    city.tapped = true;
    let city_id = city.card.id;
    game.battlefield.push(city);
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::One.index()]
        .hand
        .push(card(10_001, cards::COUNTERSPELL, PlayerId::One));

    game.turn += 1;
    game.step = Step::Upkeep;
    game.handle_upkeep_triggers();
    // The trigger reaches the stack and then resolves; the choice comes with
    // its resolution.
    let decision = advance_to_prompt(
        &mut game,
        PlayerId::One,
        "At the beginning of your upkeep, you may exile a card from your hand. If you do, untap this land.",
    );
    let yes = decision
        .options
        .iter()
        .find(|option| option.id != 0)
        .expect("accepting is offered")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![yes],
        },
    )
    .unwrap();
    drain_pending(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == city_id)
            .expect("still there")
            .tapped,
        "paying a card untapped it",
    );
    assert!(
        game.players[PlayerId::One.index()].hand.is_empty(),
        "and the card is gone",
    );
}

/// Treva's Ruins pays for its colours with the land drop before it.
#[test]
fn trevas_ruins_returns_a_land_or_sacrifices_itself() {
    let played = |pay: bool| {
        let mut game = ready_game();
        let island = creature(10_001, cards::ISLAND, PlayerId::One);
        let island_id = island.card.id;
        game.battlefield.push(island);
        let ruins = card(10_000, cards::TREVAS_RUINS, PlayerId::One);
        let ruins_card = ruins.id;
        game.players[PlayerId::One.index()].hand.push(ruins);
        game.priority = PlayerId::One;
        game.apply(
            PlayerId::One,
            Action::PlayLand {
                card: ruins_card,
                option: PlayOptionId::DEFAULT,
            },
        )
        .expect("the Lair can be played");
        pass_priority_pair(&mut game);

        let decision = game
            .observe(PlayerId::One)
            .decision
            .expect("the Lair asks for its land");
        let option = if pay {
            decision
                .options
                .iter()
                .find(|option| option.card.is_some_and(|(card, _)| card == island_id))
                .expect("the Island can pay")
                .id
        } else {
            0
        };
        game.apply(
            PlayerId::One,
            Action::ChooseDecision {
                decision: decision.id,
                options: vec![option],
            },
        )
        .unwrap();
        drain_pending(&mut game);
        game
    };

    let paid = played(true);
    assert!(
        paid.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::TREVAS_RUINS),
        "the Lair stayed",
    );
    assert!(
        paid.players[PlayerId::One.index()]
            .hand
            .iter()
            .any(|card| card.definition == cards::ISLAND),
        "and the Island went back to hand",
    );

    let declined = played(false);
    assert!(
        !declined
            .battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::TREVAS_RUINS),
        "declining sacrificed it",
    );
    assert!(
        declined
            .battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::ISLAND),
        "and the Island stayed put",
    );
}
