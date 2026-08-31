//! Chrome Mox: a free artifact whose cost is a card, paid in advance and in
//! full.

use super::*;

/// The Mox in hand, with `hand` beside it.
fn staged(hand: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let mox = game
        .build_zone(PlayerId::One, &[cards::CHROME_MOX])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let mox_id = mox.id;
    game.players[0].hand.push(mox);
    for definition in hand {
        let card = game
            .build_zone(PlayerId::One, &[*definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        game.players[0].hand.push(card);
    }
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, mox_id)
}

/// Casts it and answers the imprint with `wanted`, or declines if `None`.
fn cast_imprinting(
    game: &mut Game,
    mox: GameObjectId,
    wanted: Option<CardDefinitionId>,
) -> GameObjectId {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == mox))
        .expect("it costs nothing");
    game.apply(PlayerId::One, action).expect("it is cast");
    for _ in 0..16 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options: Vec<_> = decision
                .options
                .iter()
                .filter(|option| {
                    wanted.is_some_and(|wanted| {
                        matches!(
                            option.card,
                            Some((_, ObjectCharacteristics::Card { definition, .. }))
                                if definition == wanted
                        )
                    })
                })
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
    game.check_state_based_actions();
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == ObjectKind::Card(cards::CHROME_MOX))
        .expect("the Mox arrived")
        .card
        .id
}

/// Every colour this Mox is offering, in colour order.
fn offered_colors(game: &Game, mox: GameObjectId) -> Vec<ManaColor> {
    let mut colors = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::ActivateManaAbility { source, color, .. } if source == mox => Some(color),
            _ => None,
        })
        .collect::<Vec<_>>();
    colors.sort_unstable();
    colors.dedup();
    colors
}

/// Imprinting a green card makes it a green Mox.
#[test]
fn it_makes_the_imprinted_cards_colour() {
    let (mut game, mox) = staged(&[cards::GIANT_GROWTH]);

    let permanent = cast_imprinting(&mut game, mox, Some(cards::GIANT_GROWTH));

    assert_eq!(offered_colors(&game, permanent), vec![ManaColor::Green]);
    assert!(
        game.players[0]
            .exile
            .iter()
            .any(|card| card.definition == cards::GIANT_GROWTH),
        "and the card is gone from hand for good",
    );
    assert!(
        !game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::GIANT_GROWTH),
    );
}

/// A gold card makes both of its colours.
#[test]
fn a_two_colour_card_makes_both() {
    let (mut game, mox) = staged(&[cards::OKO_THIEF_OF_CROWNS]);

    let permanent = cast_imprinting(&mut game, mox, Some(cards::OKO_THIEF_OF_CROWNS));

    assert_eq!(
        offered_colors(&game, permanent),
        vec![ManaColor::Blue, ManaColor::Green],
    );
}

/// "You may": declining leaves a Mox that makes nothing at all.
#[test]
fn declining_leaves_it_making_nothing() {
    let (mut game, mox) = staged(&[cards::GIANT_GROWTH]);

    let permanent = cast_imprinting(&mut game, mox, None);

    assert!(offered_colors(&game, permanent).is_empty());
    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::GIANT_GROWTH),
        "and the card stayed in hand",
    );
}

/// Artifacts and lands are not on offer, which is what keeps it from being
/// free twice over.
#[test]
fn it_cannot_imprint_an_artifact_or_a_land() {
    let (mut game, mox) = staged(&[cards::FOREST, cards::MANIFOLD_KEY, cards::GIANT_GROWTH]);
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == mox))
        .expect("it costs nothing");
    game.apply(PlayerId::One, action).expect("it is cast");
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
        .expect("the imprint asks");
    let offered = decision
        .options
        .iter()
        .filter_map(|option| match option.card {
            Some((_, ObjectCharacteristics::Card { definition, .. })) => Some(definition),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        offered,
        vec![cards::GIANT_GROWTH],
        "only the nonartifact, nonland card",
    );
}

/// Tapping it makes exactly one mana of the imprinted colour.
#[test]
fn tapping_it_makes_one_mana() {
    let (mut game, mox) = staged(&[cards::GIANT_GROWTH]);
    let permanent = cast_imprinting(&mut game, mox, Some(cards::GIANT_GROWTH));

    let green = Action::ActivateManaAbility {
        source: permanent,
        ability: mana_ability_for(&game, permanent, ManaColor::Green),
        color: ManaColor::Green,
        counters_removed: None,
        cost_object: None,
        combination: None,
        triggered_mana: None,
    };
    game.apply(PlayerId::One, green).expect("it taps");

    assert_eq!(game.players[0].mana_pool.green, 1);
    assert_eq!(game.players[0].mana_pool.total(), 1);
}

/// "If the exiled card is colourless, it can't add mana", and the ability
/// can never add {C} either: a Mox with Emrakul under it is a Mox that
/// makes nothing, exactly like one that imprinted nothing at all.
#[test]
fn a_colourless_imprint_makes_nothing() {
    let (mut game, mox) = staged(&[cards::EMRAKUL_THE_AEONS_TORN]);
    let permanent = cast_imprinting(&mut game, mox, Some(cards::EMRAKUL_THE_AEONS_TORN));

    assert!(
        game.players[0]
            .exile
            .iter()
            .any(|card| card.definition == cards::EMRAKUL_THE_AEONS_TORN),
        "it was imprinted, so the question is what its colours are",
    );
    assert_eq!(
        offered_colors(&game, permanent),
        Vec::new(),
        "a colourless card names no colour, and {{C}} is not among them",
    );
}

/// The imprint is the price and it is not refunded: answering the Mox
/// leaves the card exiled where it lies.
#[test]
fn killing_the_mox_does_not_hand_the_card_back() {
    let (mut game, mox) = staged(&[cards::GIANT_GROWTH]);
    let permanent = cast_imprinting(&mut game, mox, Some(cards::GIANT_GROWTH));

    game.move_permanents_to_graveyard(&[permanent]);
    game.check_state_based_actions();
    drain_pending(&mut game);

    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::CHROME_MOX),
        "the Mox is answered",
    );
    assert!(
        game.players[0]
            .exile
            .iter()
            .any(|card| card.definition == cards::GIANT_GROWTH),
        "and the card it ate is still in exile",
    );
    assert!(game.players[0].hand.is_empty(), "nothing came back to hand");
}

/// Each Mox imprints for itself: two of them are two cards eaten and two
/// colours made, one apiece.
#[test]
fn a_second_mox_imprints_on_its_own_account() {
    let (mut game, first) = staged(&[cards::GIANT_GROWTH, cards::LIGHTNING_BOLT]);
    let second = game
        .build_zone(PlayerId::One, &[cards::CHROME_MOX])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let second_id = second.id;
    game.players[0].hand.push(second);

    let green = cast_imprinting(&mut game, first, Some(cards::GIANT_GROWTH));
    // The shared helper hands back whichever Mox it finds first, so the
    // second one is the permanent that was not there before.
    cast_imprinting(&mut game, second_id, Some(cards::LIGHTNING_BOLT));
    let red = game
        .battlefield
        .iter()
        .filter(|permanent| permanent.card.definition == ObjectKind::Card(cards::CHROME_MOX))
        .map(|permanent| permanent.card.id)
        .find(|id| *id != green)
        .expect("the second Mox arrived");

    assert_eq!(offered_colors(&game, green), vec![ManaColor::Green]);
    assert_eq!(
        offered_colors(&game, red),
        vec![ManaColor::Red],
        "the second one reads its own imprint, not the first one's",
    );
    assert_eq!(
        game.players[0].exile.len(),
        2,
        "and each of them ate a card",
    );
}
