//! Solitude: a free Swords to Plowshares at instant speed for two white
//! cards, and a lifelinking 3/2 on the turns five mana is available.

use super::*;

/// Player One holding Solitude and `mine`, with `theirs` on the battlefield
/// opposite.
fn staged(mine: &[CardDefinitionId], theirs: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[1].hand.clear();
    for definition in theirs {
        game.put_onto_battlefield(PlayerId::Two, *definition)
            .expect("cataloged");
    }
    drain_pending(&mut game);
    let solitude = game
        .build_zone(PlayerId::One, &[cards::SOLITUDE])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let solitude_id = solitude.id;
    game.players[0].hand.push(solitude);
    for definition in mine {
        let card = game
            .build_zone(PlayerId::One, &[*definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        game.players[0].hand.push(card);
    }
    game.players[0].life = 20;
    game.players[1].life = 20;
    game.turns_started = [1, 1];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, solitude_id)
}

/// Casts Solitude, by evoke when `evoked` and for its mana cost otherwise,
/// then answers the trigger by naming `wanted` -- or declining, if it is
/// not on offer.
fn cast(game: &mut Game, solitude: GameObjectId, evoked: bool, wanted: Option<GameObjectId>) {
    if !evoked {
        game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 5);
    }
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == solitude && choices.costs().alternative().is_some() == evoked
            }
            _ => false,
        })
        .expect("that way of casting it is offered");
    game.apply(PlayerId::One, action).expect("it is castable");
    settle(game, wanted);
}

fn settle(game: &mut Game, wanted: Option<GameObjectId>) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let mut options = wanted
                .into_iter()
                .flat_map(|wanted| {
                    decision
                        .options
                        .iter()
                        .filter(move |option| option.card.is_some_and(|(id, _)| id == wanted))
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

fn theirs(game: &Game, definition: CardDefinitionId) -> Option<&Permanent> {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == definition)
}

/// Evoked: a white card out of your hand, their creature exiled, its
/// controller paid in life, and Solitude herself sacrificed.
#[test]
fn evoking_her_exiles_a_creature_and_costs_two_cards() {
    let (mut game, solitude) = staged(&[cards::SWORDS_TO_PLOWSHARES], &[cards::GRIZZLY_BEARS]);
    let bears = theirs(&game, cards::GRIZZLY_BEARS)
        .expect("their creature is out")
        .card
        .id;

    cast(&mut game, solitude, true, Some(bears));

    assert!(
        game.players[1]
            .exile
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS),
        "exiled rather than destroyed",
    );
    assert_eq!(game.players[1].life, 22, "two power is two life for them");
    assert!(
        game.players[0]
            .exile
            .iter()
            .any(|card| card.definition == cards::SWORDS_TO_PLOWSHARES),
        "the white card paid for it",
    );
    assert!(
        game.battlefield.is_empty(),
        "and an evoked Solitude sacrifices herself",
    );
}

/// Cast for her mana cost instead, she stays: a 3/2 with lifelink.
#[test]
fn paying_the_mana_leaves_the_body() {
    let (mut game, solitude) = staged(&[], &[cards::GRIZZLY_BEARS]);
    let bears = theirs(&game, cards::GRIZZLY_BEARS)
        .expect("their creature is out")
        .card
        .id;

    cast(&mut game, solitude, false, Some(bears));

    assert!(
        theirs(&game, cards::GRIZZLY_BEARS).is_none(),
        "still exiled"
    );
    let body = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::SOLITUDE)
        .expect("she stayed");
    assert_eq!(game.power(body), Some(3));
    assert_eq!(game.toughness(body), Some(2));
    assert!(
        game.permanent_has_executable_keyword(body, KeywordAbility::Lifelink),
        "and her damage gains life",
    );
}

/// The life is read off the power the creature had as it left, however big
/// that was.
#[test]
fn the_life_matches_the_creature() {
    let (mut game, solitude) = staged(&[cards::SWORDS_TO_PLOWSHARES], &[cards::GRAVE_TITAN]);
    let titan = theirs(&game, cards::GRAVE_TITAN)
        .expect("their creature is out")
        .card
        .id;

    cast(&mut game, solitude, true, Some(titan));

    assert!(
        theirs(&game, cards::GRAVE_TITAN).is_none(),
        "the 6/6 is gone"
    );
    assert_eq!(game.players[1].life, 26, "and six power is six life");
}

/// "Up to one": with nothing worth exiling, the trigger may name nothing at
/// all, and Solitude is still sacrificed.
#[test]
fn she_may_exile_nothing() {
    let (mut game, solitude) = staged(&[cards::SWORDS_TO_PLOWSHARES], &[]);

    cast(&mut game, solitude, true, None);

    assert_eq!(game.players[1].life, 20, "nobody gained anything");
    assert!(game.battlefield.is_empty(), "she is gone all the same");
}

/// "Other": hard cast with no other creature anywhere, she cannot name
/// herself, so nothing is exiled and she survives her own trigger.
#[test]
fn she_cannot_exile_herself() {
    let (mut game, solitude) = staged(&[], &[]);

    cast(&mut game, solitude, false, None);

    let body = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::SOLITUDE)
        .expect("she is still there");
    assert_eq!(game.power(body), Some(3));
    assert_eq!(game.players[0].life, 20, "and nothing gained her any life");
}

/// Flash: she answers a creature on their turn, which is what the free cast
/// is for.
#[test]
fn she_comes_down_on_their_turn() {
    let (mut game, solitude) = staged(&[cards::SWORDS_TO_PLOWSHARES], &[cards::GRIZZLY_BEARS]);
    game.active_player = PlayerId::Two;
    game.priority = PlayerId::One;
    let bears = theirs(&game, cards::GRIZZLY_BEARS)
        .expect("their creature is out")
        .card
        .id;

    cast(&mut game, solitude, true, Some(bears));

    assert!(
        game.players[1]
            .exile
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS),
        "flash is what lets the free cast answer an attacker",
    );
}
