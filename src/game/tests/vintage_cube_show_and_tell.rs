//! Show and Tell: three mana to skip the mana cost of the biggest thing in
//! your deck, and to let them do it too.

use super::*;

/// Show and Tell in hand with the mana for it, `mine` and `theirs` in the
/// two hands.
fn staged(mine: &[CardDefinitionId], theirs: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[1].hand.clear();
    for (seat, definitions) in [(PlayerId::One, mine), (PlayerId::Two, theirs)] {
        for definition in definitions {
            let card = game
                .build_zone(seat, &[*definition])
                .expect("cataloged")
                .into_iter()
                .next()
                .expect("one card");
            game.players[seat.index()].hand.push(card);
        }
    }
    let spell = game
        .build_zone(PlayerId::One, &[cards::SHOW_AND_TELL])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = spell.id;
    game.players[0].hand.push(spell);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);
    game.turns_started = [2, 1];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, id)
}

/// Casts it and answers each hand choice by naming `wanted` when that
/// player holds it, and declining otherwise.
fn cast_putting_down(game: &mut Game, spell: GameObjectId, wanted: &[CardDefinitionId]) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == spell))
        .expect("three mana casts it");
    game.apply(PlayerId::One, action).expect("it is cast");
    for _ in 0..16 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .filter(|option| {
                    option.card.is_some_and(|(_, characteristics)| {
                        characteristics
                            .card_definition()
                            .is_some_and(|definition| wanted.contains(&definition))
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
            .expect("the choice accepts a card it offered, or none");
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

fn controller_of(game: &Game, definition: CardDefinitionId) -> Option<PlayerId> {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == definition)
        .map(|permanent| permanent.controller)
}

/// Both players put something down, each under their own control.
#[test]
fn each_player_puts_a_card_onto_the_battlefield() {
    let (mut game, spell) = staged(&[cards::SERRA_ANGEL], &[cards::GRIZZLY_BEARS]);

    cast_putting_down(
        &mut game,
        spell,
        &[cards::SERRA_ANGEL, cards::GRIZZLY_BEARS],
    );

    assert_eq!(
        controller_of(&game, cards::SERRA_ANGEL),
        Some(PlayerId::One),
        "yours is yours",
    );
    assert_eq!(
        controller_of(&game, cards::GRIZZLY_BEARS),
        Some(PlayerId::Two),
        "and theirs is theirs",
    );
}

/// A land counts, and so does an artifact: it is a permanent card of four
/// named types rather than a creature.
#[test]
fn a_land_is_one_of_the_things_it_names() {
    let (mut game, spell) = staged(&[cards::FOREST], &[cards::SOL_RING]);

    cast_putting_down(&mut game, spell, &[cards::FOREST, cards::SOL_RING]);

    assert_eq!(controller_of(&game, cards::FOREST), Some(PlayerId::One));
    assert_eq!(controller_of(&game, cards::SOL_RING), Some(PlayerId::Two));
}

/// "May": declining leaves the card in hand.
#[test]
fn a_player_may_decline() {
    let (mut game, spell) = staged(&[cards::SERRA_ANGEL], &[cards::GRIZZLY_BEARS]);

    // Only the Angel is wanted, so the other player is offered their Bears
    // and takes none of it.
    cast_putting_down(&mut game, spell, &[cards::SERRA_ANGEL]);

    assert_eq!(
        controller_of(&game, cards::SERRA_ANGEL),
        Some(PlayerId::One)
    );
    assert!(
        controller_of(&game, cards::GRIZZLY_BEARS).is_none(),
        "they kept theirs",
    );
    assert!(
        game.players[1]
            .hand
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS),
        "still in hand",
    );
}

/// A hand with nothing it names is never asked, and the spell still
/// resolves.
#[test]
fn a_hand_with_nothing_it_names_is_skipped() {
    let (mut game, spell) = staged(&[cards::SERRA_ANGEL], &[cards::LIGHTNING_BOLT]);

    cast_putting_down(&mut game, spell, &[cards::SERRA_ANGEL]);

    assert_eq!(
        controller_of(&game, cards::SERRA_ANGEL),
        Some(PlayerId::One)
    );
    assert!(
        game.players[1]
            .hand
            .iter()
            .any(|card| card.definition == cards::LIGHTNING_BOLT),
        "an instant is not one of the four types",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::SHOW_AND_TELL),
        "and the spell resolved",
    );
}

/// "The current player chooses first, then each other player chooses in turn
/// order. After all choices are made, the cards are put onto the battlefield
/// simultaneously." So the first question is the caster's, and answering it
/// puts nothing anywhere: the board is still empty while the other player
/// decides.
#[test]
fn the_caster_chooses_first_and_nothing_lands_until_both_have() {
    let (mut game, spell) = staged(&[cards::GRIZZLY_BEARS], &[cards::SERRA_ANGEL]);

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == spell))
        .expect("three mana casts it");
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

    let first = game
        .pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("somebody is asked what to put down");
    assert_eq!(
        first.player,
        PlayerId::One,
        "the player whose turn it is chooses first",
    );
    let bears = first
        .options
        .iter()
        .find(|option| {
            option.card.is_some_and(|(_, characteristics)| {
                characteristics.card_definition() == Some(cards::GRIZZLY_BEARS)
            })
        })
        .expect("the Bears are on offer")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: first.id,
            options: vec![bears],
        },
    )
    .expect("choosing them is legal");

    assert!(
        game.battlefield.is_empty(),
        "nothing is on the battlefield while the other player is still choosing",
    );
    let second = game
        .pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("and then they are asked");
    assert_eq!(second.player, PlayerId::Two, "in turn order after you");
}

/// "An artifact, creature, enchantment, or land card": a planeswalker is
/// none of those, and neither is an instant, so a hand of them is not asked.
#[test]
fn it_names_four_types_and_no_others() {
    let (mut game, spell) = staged(
        &[cards::WRENN_AND_SIX, cards::LIGHTNING_BOLT],
        &[cards::GRIZZLY_BEARS],
    );

    cast_putting_down(
        &mut game,
        spell,
        &[cards::WRENN_AND_SIX, cards::LIGHTNING_BOLT],
    );

    assert!(
        controller_of(&game, cards::WRENN_AND_SIX).is_none(),
        "a planeswalker is not one of the four",
    );
    assert!(
        controller_of(&game, cards::LIGHTNING_BOLT).is_none(),
        "and an instant is not either",
    );
    assert_eq!(
        game.players[0].hand.len(),
        2,
        "both are still in hand, the Show and Tell having left it",
    );
}
