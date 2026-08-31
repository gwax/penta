//! Shelldock Isle: a tapped Island that hides a card until the game is
//! nearly over, and then plays it for nothing.

use super::*;

/// The Isle on the battlefield with `library` stacked on top of player
/// one's library.
fn staged(library: &[CardDefinitionId], library_size: usize) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    game.players[1].library.clear();
    // Filler under the named cards, so the library is as deep as asked.
    let filler = library_size.saturating_sub(library.len());
    for index in 0..filler {
        game.players[0].library.push(card(
            105_000 + u32::try_from(index).expect("few cards"),
            cards::FOREST,
            PlayerId::One,
        ));
    }
    for (index, definition) in library.iter().rev().enumerate() {
        game.players[0].library.push(card(
            105_500 + u32::try_from(index).expect("few cards"),
            *definition,
            PlayerId::One,
        ));
    }
    for index in 0..40 {
        game.players[1]
            .library
            .push(card(106_000 + index, cards::ISLAND, PlayerId::Two));
    }
    let isle = game
        .put_onto_battlefield(PlayerId::One, cards::SHELLDOCK_ISLE)
        .expect("cataloged");
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, isle)
}

/// Answers the hideaway look by taking `wanted`.
fn hide(game: &mut Game, wanted: CardDefinitionId) {
    for _ in 0..12 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|p| p.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .filter(|option| {
                    option.card.is_some_and(|(_, characteristics)| {
                        characteristics.card_definition() == Some(wanted)
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
            .expect("the hidden card is one of the four");
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
}

fn unlock(game: &Game, isle: GameObjectId) -> Option<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == isle))
}

/// It arrives tapped, hides one of the top four, and puts the rest back.
#[test]
fn it_hides_one_of_the_top_four() {
    let (mut game, isle) = staged(
        &[
            cards::BLACK_LOTUS,
            cards::MOX_JET,
            cards::GRIZZLY_BEARS,
            cards::SERRA_ANGEL,
        ],
        30,
    );
    assert!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == isle)
            .expect("it is there")
            .tapped,
        "it enters tapped",
    );

    hide(&mut game, cards::BLACK_LOTUS);

    assert_eq!(
        game.players[0]
            .exile
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::BLACK_LOTUS],
    );
    assert_eq!(
        game.players[0].library.len(),
        29,
        "the other three went back under the library",
    );
    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { .. })),
        "hiding it is not permission to play it",
    );
}

/// With both libraries deep, the unlock is not offered.
#[test]
fn a_full_library_keeps_it_locked() {
    let (mut game, isle) = staged(&[cards::BLACK_LOTUS], 30);
    hide(&mut game, cards::BLACK_LOTUS);
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == isle)
    {
        permanent.tapped = false;
    }
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);

    assert!(
        unlock(&game, isle).is_none(),
        "thirty cards is more than twenty",
    );
}

/// Once a library is down to twenty, the hidden card may be played for free.
#[test]
fn twenty_cards_unlocks_it() {
    let (mut game, isle) = staged(&[cards::BLACK_LOTUS], 20);
    hide(&mut game, cards::BLACK_LOTUS);
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == isle)
    {
        permanent.tapped = false;
    }
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);

    let unlock = unlock(&game, isle).expect("nineteen cards is fewer than twenty");
    game.apply(PlayerId::One, unlock).expect("it activates");
    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { .. }))
        .expect("the hidden Lotus is castable now");
    game.apply(PlayerId::One, cast).expect("it is cast");
    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::BLACK_LOTUS),
        "and it cost nothing to cast",
    );
}

/// Answers the standing "play it, or decline" offer by declining.
fn decline(game: &mut Game) {
    let decision = game
        .pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("the offer is waiting");
    let options = decision
        .options
        .iter()
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
    .expect("declining is legal");
}

/// Unlocks the Isle and settles the stack, leaving the offer standing.
fn unlock_and_settle(game: &mut Game, isle: GameObjectId) {
    let action = unlock(game, isle).expect("nineteen cards is fewer than twenty");
    game.apply(PlayerId::One, action).expect("it activates");
    for _ in 0..8 {
        if !game.stack.is_empty() && game.pending_decisions.is_empty() {
            let priority = game.priority;
            if game.apply(priority, Action::PassPriority).is_err() {
                break;
            }
            continue;
        }
        break;
    }
}

/// "You may play the exiled card" is an offer that stands while the ability
/// resolves and no longer: a player who declines it does not keep the card
/// playable for the rest of the turn.
#[test]
fn declining_puts_the_card_back_out_of_reach() {
    let (mut game, isle) = staged(&[cards::BLACK_LOTUS], 20);
    hide(&mut game, cards::BLACK_LOTUS);
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == isle)
    {
        permanent.tapped = false;
    }
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);

    unlock_and_settle(&mut game, isle);
    assert!(
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { .. })),
        "the offer stands while the decision does",
    );

    decline(&mut game);

    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { .. })),
        "and is gone once it is declined",
    );
    assert_eq!(
        game.players[0]
            .exile
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::BLACK_LOTUS],
        "the card itself stays hidden where it was",
    );
}

/// Declining costs the activation rather than the card: pay the {U} and the
/// tap again and the same card is offered again.
#[test]
fn a_second_activation_offers_it_again() {
    let (mut game, isle) = staged(&[cards::BLACK_LOTUS], 20);
    hide(&mut game, cards::BLACK_LOTUS);
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == isle)
    {
        permanent.tapped = false;
    }
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 2);

    unlock_and_settle(&mut game, isle);
    decline(&mut game);
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == isle)
    {
        permanent.tapped = false;
    }

    unlock_and_settle(&mut game, isle);

    assert!(
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { .. })),
        "the same hidden card, offered a second time",
    );
}

/// "It doesn't matter which library has twenty or fewer cards in it." Their
/// deck running out unlocks your Isle as surely as yours does.
#[test]
fn their_short_library_unlocks_it_too() {
    let (mut game, isle) = staged(&[cards::BLACK_LOTUS], 30);
    hide(&mut game, cards::BLACK_LOTUS);
    game.players[1].library.truncate(20);
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == isle)
    {
        permanent.tapped = false;
    }
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);

    assert_eq!(
        game.players[0].library.len(),
        29,
        "yours is nowhere near twenty",
    );
    assert!(
        unlock(&game, isle).is_some(),
        "and theirs is what the card asks about",
    );
}

/// The half that is used every other game: it is an Island underneath.
#[test]
fn it_taps_for_blue() {
    let (mut game, isle) = staged(&[cards::BLACK_LOTUS], 30);
    hide(&mut game, cards::BLACK_LOTUS);
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == isle)
    {
        permanent.tapped = false;
    }

    game.apply(
        PlayerId::One,
        Action::ActivateManaAbility {
            source: isle,
            ability: mana_ability_for(&game, isle, ManaColor::Blue),
            color: ManaColor::Blue,
            counters_removed: None,
            cost_object: None,
            combination: None,
            triggered_mana: None,
        },
    )
    .expect("it taps for blue");

    assert_eq!(game.players[0].mana_pool.blue, 1);
    assert_eq!(game.players[0].mana_pool.total(), 1, "one mana, no more");
}

/// "Play the exiled card", not cast it: a land that was hidden is played,
/// which spends the land drop the turn it happens.
#[test]
fn a_hidden_land_is_played_and_costs_the_land_drop() {
    let (mut game, isle) = staged(&[cards::FOREST], 20);
    hide(&mut game, cards::FOREST);
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == isle)
    {
        permanent.tapped = false;
    }
    game.players[0].lands_played_this_turn = 0;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    let hidden = game.players[0].exile[0].id;

    unlock_and_settle(&mut game, isle);

    println!(
        "EXILE {:?}",
        game.players[0]
            .exile
            .iter()
            .map(|c| c.definition)
            .collect::<Vec<_>>()
    );
    println!(
        "STACK {} DECISIONS {}",
        game.stack.len(),
        game.pending_decisions.len()
    );
    for action in game.legal_actions(PlayerId::One) {
        println!("ACTION {action:?}");
    }
    let play = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::PlayLand { card, .. } if *card == hidden))
        .expect("the hidden land is played rather than cast");
    game.apply(PlayerId::One, play).expect("it is played");
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::FOREST),
        "the Forest arrived from exile",
    );
    assert!(
        game.players[0].exile.is_empty(),
        "and left the exile it was hidden in",
    );
    assert_eq!(
        game.players[0].lands_played_this_turn, 1,
        "playing a land is playing a land, however it was paid for",
    );
}

/// The land drop is the one limit the permission does not lift: with it
/// already spent there is nothing to offer, and the card stays hidden.
#[test]
fn a_hidden_land_needs_a_land_drop_left() {
    let (mut game, isle) = staged(&[cards::FOREST], 20);
    hide(&mut game, cards::FOREST);
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == isle)
    {
        permanent.tapped = false;
    }
    game.players[0].lands_played_this_turn = 1;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);

    unlock_and_settle(&mut game, isle);

    assert!(
        game.pending_decisions.is_empty(),
        "an offer nobody could take is not made",
    );
    assert_eq!(
        game.players[0]
            .exile
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::FOREST],
        "and the land is still hidden",
    );
}
