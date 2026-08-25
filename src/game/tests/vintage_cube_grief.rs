//! Grief: a Thoughtseize that costs a second black card instead of mana, and
//! a 3/2 with menace on the turns you would rather pay the mana.

use super::*;

/// Player One holding Grief and `mine`, with `theirs` in the other hand.
fn staged(mine: &[CardDefinitionId], theirs: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[1].hand.clear();
    let grief = game
        .build_zone(PlayerId::One, &[cards::GRIEF])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let grief_id = grief.id;
    game.players[0].hand.push(grief);
    for (player, cards) in [(PlayerId::One, mine), (PlayerId::Two, theirs)] {
        for definition in cards {
            let card = game
                .build_zone(player, &[*definition])
                .expect("cataloged")
                .into_iter()
                .next()
                .expect("one card");
            game.players[player.index()].hand.push(card);
        }
    }
    game.turns_started = [1, 1];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, grief_id)
}

/// Casts Grief, by evoke when `evoked` and for its mana cost otherwise, and
/// takes `wanted` with the trigger.
fn cast(game: &mut Game, grief: GameObjectId, evoked: bool, wanted: Option<CardDefinitionId>) {
    if !evoked {
        game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 4);
    }
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == grief && choices.costs().alternative().is_some() == evoked
            }
            _ => false,
        })
        .expect("that way of casting it is offered");
    game.apply(PlayerId::One, action).expect("it is castable");
    settle(game, wanted);
}

fn settle(game: &mut Game, wanted: Option<CardDefinitionId>) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let mut options = wanted
                .into_iter()
                .flat_map(|wanted| {
                    decision.options.iter().filter(move |option| {
                        option.card.is_some_and(|(_, characteristics)| {
                            characteristics.card_definition() == Some(wanted)
                        })
                    })
                })
                .map(|option| option.id)
                .take(1)
                .collect::<Vec<_>>();
            if options.is_empty() {
                options = decision
                    .options
                    .iter()
                    .map(|option| option.id)
                    .take(decision.minimum.max(1))
                    .collect();
            }
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
    game.check_state_based_actions();
}

fn their_hand(game: &Game) -> Vec<CardDefinitionId> {
    game.players[1]
        .hand
        .iter()
        .map(|card| card.definition)
        .collect()
}

/// Evoked on turn one: a black card out of your hand, a card out of theirs,
/// and Grief itself sacrificed.
#[test]
fn evoking_it_takes_a_card_and_costs_two() {
    let (mut game, grief) = staged(
        &[cards::DARK_RITUAL],
        &[cards::ANCESTRAL_RECALL, cards::SWAMP],
    );

    cast(&mut game, grief, true, Some(cards::ANCESTRAL_RECALL));

    assert_eq!(their_hand(&game), vec![cards::SWAMP], "the Recall is gone");
    assert!(
        game.players[1]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::ANCESTRAL_RECALL),
        "discarded rather than exiled",
    );
    assert!(
        game.players[0]
            .exile
            .iter()
            .any(|card| card.definition == cards::DARK_RITUAL),
        "the black card paid for it",
    );
    assert!(
        game.battlefield.is_empty(),
        "and an evoked Grief sacrifices itself",
    );
}

/// "A nonland card": their lands are not on offer, and a hand of nothing but
/// lands loses nothing.
#[test]
fn it_takes_only_nonland_cards() {
    let (mut game, grief) = staged(&[cards::DARK_RITUAL], &[cards::SWAMP, cards::MOUNTAIN]);

    cast(&mut game, grief, true, None);

    assert_eq!(
        their_hand(&game).len(),
        2,
        "nothing in that hand was takeable",
    );
}

/// Cast for its mana cost instead, it takes a card and stays: a 3/2 with
/// menace.
#[test]
fn paying_the_mana_leaves_the_body() {
    let (mut game, grief) = staged(&[], &[cards::ANCESTRAL_RECALL]);

    cast(&mut game, grief, false, Some(cards::ANCESTRAL_RECALL));

    assert!(their_hand(&game).is_empty(), "it still takes the card");
    let body = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::GRIEF)
        .expect("Grief stayed");
    assert_eq!(game.power(body), Some(3));
    assert!(
        game.permanent_has_executable_keyword(body, KeywordAbility::Menace),
        "and it is hard to block",
    );
}

/// The evoke cost is a black card: with nothing black in hand there is no
/// free cast on offer.
#[test]
fn evoke_needs_a_black_card() {
    let (game, grief) = staged(&[cards::ANCESTRAL_RECALL], &[cards::SWAMP]);

    assert!(
        !game.legal_actions(PlayerId::One).iter().any(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if *card == grief && choices.costs().alternative().is_some())
        }),
        "a blue card does not pay for it",
    );
}

/// Grief itself is a black card, but it is the spell rather than the cost:
/// evoking it with nothing else in hand is not a way to cast it for free.
#[test]
fn it_cannot_pay_for_itself() {
    let (game, grief) = staged(&[], &[cards::SWAMP]);

    assert!(
        !game.legal_actions(PlayerId::One).iter().any(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if *card == grief && choices.costs().alternative().is_some())
        }),
        "the card being cast is not in hand to be exiled",
    );
}
