//! Flash: two mana that puts anything onto the battlefield for two less, and
//! a real card in a deck that would rather the creature died anyway.

use super::*;

/// Player One holding a Flash and `hand` beside it.
fn staged(hand: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    let spell = game
        .build_zone(PlayerId::One, &[cards::FLASH])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let spell_id = spell.id;
    game.players[0].hand.push(spell);
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
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 2);
    (game, spell_id)
}

/// Casts it, naming `wanted` if it is offered, and answering the payment
/// with `pay`.
fn cast(game: &mut Game, spell: GameObjectId, wanted: Option<CardDefinitionId>, pay: bool) {
    put_on_stack(game, spell);
    settle(game, wanted, pay);
}

/// Puts it on the stack, which is where its own two mana is spent: a test
/// that cares which mana pays the creature adds that mana after this.
fn put_on_stack(game: &mut Game, spell: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == spell))
        .expect("two mana casts it");
    game.apply(PlayerId::One, action).expect("it is cast");
}

/// Resolves it, naming `wanted` if it is offered and answering the payment
/// with `pay`.
fn settle(game: &mut Game, wanted: Option<CardDefinitionId>, pay: bool) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = if let Some(option) = decision
                .options
                .iter()
                .find(|option| option.label == if pay { "Pay the cost" } else { "Decline" })
            {
                vec![option.id]
            } else {
                decision
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
                    .collect()
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
    game.check_state_based_actions();
}

fn on_battlefield(game: &Game, definition: CardDefinitionId) -> bool {
    game.battlefield
        .iter()
        .any(|permanent| permanent.card.definition == ObjectKind::Card(definition))
}

/// Declining the payment sacrifices what came in, which is the mode the
/// card is actually played for.
#[test]
fn declining_the_payment_sacrifices_it() {
    let (mut game, spell) = staged(&[cards::SERRA_ANGEL]);

    cast(&mut game, spell, Some(cards::SERRA_ANGEL), false);

    assert!(
        !on_battlefield(&game, cards::SERRA_ANGEL),
        "it was sacrificed"
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::SERRA_ANGEL),
        "and it is in the graveyard, having been on the battlefield",
    );
}

/// Paying keeps it: a five-drop for {3}{W}{W} instead of {5}, and the two
/// mana the spell cost.
#[test]
fn paying_keeps_the_creature() {
    let (mut game, spell) = staged(&[cards::SERRA_ANGEL]);
    // Serra Angel is {3}{W}{W}; reduced by {2} that is {1}{W}{W}.
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 2);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);

    cast(&mut game, spell, Some(cards::SERRA_ANGEL), true);

    assert!(on_battlefield(&game, cards::SERRA_ANGEL), "it stayed");
    assert!(
        !game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::SERRA_ANGEL),
        "and was not sacrificed",
    );
}

/// The reduction is on the generic half: without the white mana the cost
/// cannot be paid at all, however much colourless is lying around.
#[test]
fn the_coloured_pips_still_have_to_be_paid() {
    let (mut game, spell) = staged(&[cards::SERRA_ANGEL]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 9);

    cast(&mut game, spell, Some(cards::SERRA_ANGEL), true);

    assert!(
        !on_battlefield(&game, cards::SERRA_ANGEL),
        "nine colourless does not pay one generic and two white",
    );
}

/// "You may": with nothing named the spell resolves and does nothing.
#[test]
fn it_may_name_nothing() {
    let (mut game, spell) = staged(&[cards::SERRA_ANGEL]);

    cast(&mut game, spell, None, false);

    assert!(!on_battlefield(&game, cards::SERRA_ANGEL));
    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::SERRA_ANGEL),
        "the creature stayed in hand",
    );
}

/// It names creature cards and nothing else.
#[test]
fn it_offers_only_creature_cards() {
    let (mut game, spell) = staged(&[cards::SERRA_ANGEL, cards::ANCESTRAL_RECALL, cards::FOREST]);
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == spell))
        .expect("two mana casts it");
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
        .expect("it asks which creature");
    let offered = decision
        .options
        .iter()
        .filter_map(|option| match option.card {
            Some((_, ObjectCharacteristics::Card { definition, .. })) => Some(definition),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(offered, vec![cards::SERRA_ANGEL]);
}

/// "The creature enters the battlefield, so it will trigger
/// enters-the-battlefield abilities even if you choose not to pay." The
/// Battlesphere goes; the four Myr it brought stay.
#[test]
fn the_arrival_triggers_even_when_the_payment_is_declined() {
    let (mut game, spell) = staged(&[cards::MYR_BATTLESPHERE]);

    cast(&mut game, spell, Some(cards::MYR_BATTLESPHERE), false);

    assert!(
        !on_battlefield(&game, cards::MYR_BATTLESPHERE),
        "seven mana went unpaid, so it was sacrificed",
    );
    let myr = tokens::artifact_creature(&["Myr"], &[], 1, 1);
    assert_eq!(
        game.battlefield
            .iter()
            .filter(|permanent| is_token_with(permanent, myr))
            .count(),
        4,
        "its arrival trigger resolved all the same",
    );
}

/// "If the creature has {X} in its mana cost, X is considered to be 0." The
/// Rabbit's {X}{1}{W} is therefore {1}{W}, and reduced by {2} that is one
/// white mana to keep a creature that arrives with no counters at all.
#[test]
fn an_x_in_the_cost_counts_as_zero() {
    let (mut game, spell) = staged(&[cards::JACKED_RABBIT]);

    put_on_stack(&mut game, spell);
    // After the spell's own {1}{U}, so that the white is there for the
    // creature rather than being eaten by the generic half of Flash.
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    settle(&mut game, Some(cards::JACKED_RABBIT), true);

    assert!(
        on_battlefield(&game, cards::JACKED_RABBIT),
        "one white paid for it, so it stayed",
    );
    assert_eq!(
        game.players[0].mana_pool.total(),
        0,
        "and that one white is all it took",
    );
    let rabbit = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == ObjectKind::Card(cards::JACKED_RABBIT))
        .expect("it stayed");
    assert_eq!(
        (game.power(rabbit), game.toughness(rabbit)),
        (Some(1), Some(2)),
        "Ravenous read the same zero, so it brought no counters",
    );
}
