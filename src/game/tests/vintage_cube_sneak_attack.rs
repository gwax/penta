//! Sneak Attack: one red mana per creature, and the deck stops casting
//! things.

use super::*;

/// The enchantment on the battlefield, a fat creature in hand, and `mana`
/// red available.
fn staged(mana: u16) -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let sneak = game
        .put_onto_battlefield(PlayerId::One, cards::SNEAK_ATTACK)
        .expect("cataloged");
    let dragon = game
        .build_zone(PlayerId::One, &[cards::SHIVAN_DRAGON])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let dragon_id = dragon.id;
    game.players[0].hand.push(dragon);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, mana);
    drain_pending(&mut game);
    (game, sneak, dragon_id)
}

/// Activates Sneak Attack, taking `wanted` when the choice is offered.
fn sneak_in(game: &mut Game, sneak: GameObjectId, wanted: Option<CardDefinitionId>) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == sneak))
        .expect("one red activates it");
    game.apply(PlayerId::One, action).expect("it activates");
    for _ in 0..16 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = wanted
                .and_then(|wanted| {
                    decision.options.iter().find(|option| {
                        option.card.is_some_and(|(_, characteristics)| {
                            characteristics.card_definition() == Some(wanted)
                        })
                    })
                })
                .map(|option| vec![option.id])
                .unwrap_or_default();
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the choice accepts what it offered");
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

fn dragon_on_battlefield(game: &Game) -> Option<&Permanent> {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::SHIVAN_DRAGON)
}

/// It puts the creature down and it can attack at once.
#[test]
fn it_cheats_a_creature_in_with_haste() {
    let (mut game, sneak, _) = staged(1);

    sneak_in(&mut game, sneak, Some(cards::SHIVAN_DRAGON));

    let dragon = dragon_on_battlefield(&game).expect("the Dragon is out");
    assert!(
        game.permanent_has_executable_keyword(dragon, KeywordAbility::Haste),
        "and it can attack the turn it arrives",
    );
    assert!(
        game.players[0].hand.is_empty(),
        "it came out of the hand rather than being cast",
    );
}

/// The end step takes it back: the whole point of the card is that the
/// creature is borrowed.
#[test]
fn the_end_step_sacrifices_it() {
    let (mut game, sneak, _) = staged(1);
    sneak_in(&mut game, sneak, Some(cards::SHIVAN_DRAGON));
    assert!(dragon_on_battlefield(&game).is_some());

    game.step = Step::End;
    game.begin_step_triggers();
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert!(
        dragon_on_battlefield(&game).is_none(),
        "the Dragon was sacrificed",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::SHIVAN_DRAGON),
        "and it went to the graveyard",
    );
}

/// One red buys one creature, and the enchantment stays for the next one.
#[test]
fn every_activation_costs_one_red() {
    let (mut game, sneak, _) = staged(2);
    let second = game
        .build_zone(PlayerId::One, &[cards::GRIZZLY_BEARS])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    game.players[0].hand.push(second);

    sneak_in(&mut game, sneak, Some(cards::SHIVAN_DRAGON));
    sneak_in(&mut game, sneak, Some(cards::GRIZZLY_BEARS));

    assert!(dragon_on_battlefield(&game).is_some());
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::GRIZZLY_BEARS),
        "both creatures are out",
    );
    assert_eq!(game.players[0].mana_pool.red, 0, "for two red");
}

/// "You may": declining puts nothing down, and the mana is still spent.
#[test]
fn declining_puts_nothing_down() {
    let (mut game, sneak, _) = staged(1);

    sneak_in(&mut game, sneak, None);

    assert!(dragon_on_battlefield(&game).is_none(), "nothing arrived");
    assert_eq!(game.players[0].hand.len(), 1, "the Dragon is still in hand");
    assert_eq!(game.players[0].mana_pool.red, 0, "and the red is gone");
}

/// A land in hand is not a creature card: it is never on offer.
#[test]
fn only_creature_cards_are_offered() {
    let (mut game, sneak, _) = staged(1);
    let mountain = game
        .build_zone(PlayerId::One, &[cards::MOUNTAIN])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    game.players[0].hand.push(mountain);

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == sneak))
        .expect("one red activates it");
    game.apply(PlayerId::One, action).expect("it activates");
    for _ in 0..8 {
        if !game.pending_decisions.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }

    let offered = game
        .pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("the choice is waiting")
        .options
        .iter()
        .filter_map(|option| option.card.map(|(_, characteristics)| characteristics))
        .filter_map(ObjectCharacteristics::card_definition)
        .collect::<Vec<_>>();

    assert_eq!(
        offered,
        vec![cards::SHIVAN_DRAGON],
        "the creature and not the land",
    );
}

/// "You sacrifice the creature only if you still control it. If that
/// creature has left the battlefield, even if it came back, you don't
/// sacrifice it." An Ephemerate in response to nothing at all is enough:
/// what returns is a new object the delayed sacrifice has never heard of.
#[test]
fn a_creature_that_left_and_returned_is_not_sacrificed() {
    let (mut game, sneak, _) = staged(1);
    let ephemerate = game
        .build_zone(PlayerId::One, &[cards::EPHEMERATE])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let ephemerate_id = ephemerate.id;
    game.players[0].hand.push(ephemerate);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);

    sneak_in(&mut game, sneak, Some(cards::SHIVAN_DRAGON));
    let borrowed = dragon_on_battlefield(&game)
        .expect("the Dragon is here")
        .card
        .id;

    let blink = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == ephemerate_id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(borrowed))
            }
            _ => false,
        })
        .expect("one white blinks it");
    game.apply(PlayerId::One, blink).expect("it is cast");
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
    let returned = dragon_on_battlefield(&game)
        .expect("the Dragon came back")
        .card
        .id;
    assert_ne!(returned, borrowed, "as a new object");

    game.step = Step::End;
    game.begin_step_triggers();
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert!(
        dragon_on_battlefield(&game).is_some(),
        "and the end step has nothing to collect",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .all(|card| card.definition != cards::SHIVAN_DRAGON),
        "the Dragon is not in the graveyard: it was never sacrificed",
    );
}

/// "At the beginning of the next end step" is read from where the ability
/// resolved. Sneaking a creature in during an end step that has already
/// begun means the next such beginning is a turn away, so the Dragon lives
/// through the opponent's whole turn and is sacrificed at their end step.
#[test]
fn one_sneaked_in_during_an_end_step_lives_until_the_next_one() {
    let (mut game, sneak, _) = staged(1);
    game.step = Step::End;
    game.begin_step_triggers();
    drain_pending(&mut game);

    sneak_in(&mut game, sneak, Some(cards::SHIVAN_DRAGON));
    assert!(
        dragon_on_battlefield(&game).is_some(),
        "it arrived after this end step had already begun",
    );

    // Finishing the end step it arrived in takes nothing: that beginning
    // came and went before the delayed trigger existed.
    game.check_state_based_actions();
    assert!(
        dragon_on_battlefield(&game).is_some(),
        "so this end step is not the one that claims it",
    );

    // The opponent's turn, all the way to their end step.
    game.turn += 1;
    game.active_player = PlayerId::Two;
    game.turns_started[PlayerId::Two.index()] += 1;
    for step in [Step::Upkeep, Step::Draw, Step::PrecombatMain] {
        game.step = step;
        game.begin_step_triggers();
        drain_pending(&mut game);
        game.check_state_based_actions();
        assert!(
            dragon_on_battlefield(&game).is_some(),
            "it is still borrowed at {step:?}",
        );
    }

    game.step = Step::End;
    game.begin_step_triggers();
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert!(
        dragon_on_battlefield(&game).is_none(),
        "and the next end step is the one that takes it",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::SHIVAN_DRAGON),
        "sacrificed by its own controller, wherever the turn belongs",
    );
}
