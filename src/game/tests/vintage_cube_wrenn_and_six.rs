//! Wrenn and Six: a two-mana planeswalker that buys back a land every turn,
//! pings on the way down, and eventually hands the graveyard retrace.

use super::*;

fn staged(graveyard: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    for (index, definition) in graveyard.iter().enumerate() {
        game.players[0].graveyard.push(card(
            110_000 + u32::try_from(index).expect("few cards"),
            *definition,
            PlayerId::One,
        ));
    }
    let wrenn = game
        .put_onto_battlefield(PlayerId::One, cards::WRENN_AND_SIX)
        .expect("cataloged");
    drain_pending(&mut game);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, wrenn)
}

/// The first legal activation of one of Wrenn's printed loyalty abilities,
/// counted the way they are printed: plus, minus, ultimate.
fn loyalty_action(game: &Game, wrenn: GameObjectId, ability: u8) -> Option<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source,
                ability: AbilityOrigin::Printed { ability: id, .. },
                ..
            } => *source == wrenn && *id == AbilityId(ability),
            _ => false,
        })
}

fn settle(game: &mut Game) {
    for _ in 0..12 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .map(|option| option.id)
                .take(decision.minimum.max(1).min(decision.maximum))
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
}

/// The plus buys back a land and nothing else.
#[test]
fn the_plus_returns_a_land() {
    let (mut game, wrenn) = staged(&[cards::MOUNTAIN, cards::LIGHTNING_BOLT]);

    let plus = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } => {
                *source == wrenn
                    && targets
                        .iter()
                        .flat_map(crate::casting::TargetSelection::targets)
                        .count()
                        == 1
            }
            _ => false,
        })
        .expect("a land in the graveyard is a legal target");
    game.apply(PlayerId::One, plus).expect("it activates");
    settle(&mut game);

    assert_eq!(
        game.players[0]
            .hand
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::MOUNTAIN],
        "the Bolt was never a legal target",
    );
}

/// The minus pings, and costs a loyalty counter.
#[test]
fn the_minus_deals_one() {
    let (mut game, wrenn) = staged(&[]);
    let bears = creature(110_500, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);

    let ping = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } => {
                *source == wrenn
                    && targets
                        .iter()
                        .flat_map(crate::casting::TargetSelection::targets)
                        .any(|chosen| *chosen == Target::Permanent(bears_id))
            }
            _ => false,
        })
        .expect("any target includes a creature");
    game.apply(PlayerId::One, ping).expect("it activates");
    settle(&mut game);

    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == bears_id)
            .expect("a 2/2 survives one")
            .damage,
        1,
    );
    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == wrenn)
            .expect("she is there")
            .counters(CounterKind::Loyalty),
        2,
        "three loyalty minus one",
    );
}

/// The ultimate's emblem hands the graveyard retrace: an instant there may
/// be cast for its cost plus a discarded land.
#[test]
fn the_emblem_grants_retrace() {
    let (mut game, wrenn) = staged(&[cards::LIGHTNING_BOLT]);
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == wrenn)
    {
        permanent.set_counters(CounterKind::Loyalty, 7);
    }
    let bolt = game.players[0].graveyard[0].id;
    game.players[0]
        .hand
        .push(card(110_600, cards::MOUNTAIN, PlayerId::One));
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);

    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == bolt)),
        "a Bolt in the graveyard is not castable before the emblem",
    );

    let ultimate = loyalty_action(&game, wrenn, 2).expect("seven loyalty pays for it");
    game.apply(PlayerId::One, ultimate).expect("it activates");
    settle(&mut game);

    let retrace = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == bolt))
        .expect("with a land in hand and a red mana, retrace is payable");
    game.apply(PlayerId::One, retrace).expect("it casts");
    settle(&mut game);

    assert!(
        game.players[0].hand.is_empty(),
        "the land was discarded to pay for it",
    );
    assert_eq!(
        game.players[0]
            .graveyard
            .iter()
            .filter(|card| card.definition == cards::LIGHTNING_BOLT)
            .count(),
        1,
        "retrace leaves the card where it was cast from, to be cast again",
    );
}

/// "When a spell you cast with retrace resolves or is countered, it's put
/// back into your graveyard. You may use the retrace ability to cast it
/// again." Two lands in hand is two Bolts, off one card.
#[test]
fn a_retraced_spell_can_be_retraced_again() {
    let (mut game, wrenn) = staged(&[cards::LIGHTNING_BOLT]);
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == wrenn)
    {
        permanent.set_counters(CounterKind::Loyalty, 7);
    }
    for index in 0..2 {
        game.players[0]
            .hand
            .push(card(110_600 + index, cards::MOUNTAIN, PlayerId::One));
    }
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 2);
    let event_start = game.events().len();

    let ultimate = loyalty_action(&game, wrenn, 2).expect("seven loyalty pays for it");
    game.apply(PlayerId::One, ultimate).expect("it activates");
    settle(&mut game);

    for attempt in 0..2 {
        // Going to the stack and back makes it a new object each time, so the
        // second cast is found by what the card is rather than by which one
        // it was.
        let bolt = game.players[0]
            .graveyard
            .iter()
            .find(|card| card.definition == cards::LIGHTNING_BOLT)
            .unwrap_or_else(|| panic!("the Bolt is in the graveyard before cast {attempt}"))
            .id;
        let retrace = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == bolt))
            .unwrap_or_else(|| panic!("a land and a red mana pay for cast {attempt}"));
        game.apply(PlayerId::One, retrace).expect("it casts");
        settle(&mut game);
    }

    assert_eq!(
        game.events()[event_start..]
            .iter()
            .filter(|event| matches!(
                event,
                GameEvent::SpellResolved {
                    definition: cards::LIGHTNING_BOLT,
                    ..
                }
            ))
            .count(),
        2,
        "the same Bolt resolved twice, for one land each",
    );
    assert!(
        game.players[0].hand.is_empty(),
        "both lands went to pay for it",
    );
    assert_eq!(
        game.players[0]
            .graveyard
            .iter()
            .filter(|card| card.definition == cards::LIGHTNING_BOLT)
            .count(),
        1,
        "and it is back in the graveyard, ready for a third land",
    );
}

/// "Up to one target": with nothing to buy back the plus is still an
/// activation, and Wrenn still ticks up.
#[test]
fn the_plus_ticks_up_with_an_empty_graveyard() {
    let (mut game, wrenn) = staged(&[]);

    let plus = loyalty_action(&game, wrenn, 0).expect("a plus needs no target");
    game.apply(PlayerId::One, plus).expect("it activates");
    settle(&mut game);

    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == wrenn)
            .expect("she is still there")
            .counters(CounterKind::Loyalty),
        4,
        "three loyalty plus one, having returned nothing",
    );
}

/// The emblem says instant and sorcery cards in *your* graveyard: a creature
/// card of yours and an instant of theirs are both out of reach.
#[test]
fn the_emblem_reaches_neither_creatures_nor_the_other_graveyard() {
    let (mut game, wrenn) = staged(&[cards::GRIZZLY_BEARS]);
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == wrenn)
    {
        permanent.set_counters(CounterKind::Loyalty, 7);
    }
    let bears = game.players[0].graveyard[0].id;
    game.players[1].graveyard.clear();
    game.players[1]
        .graveyard
        .push(card(110_700, cards::LIGHTNING_BOLT, PlayerId::Two));
    let theirs = game.players[1].graveyard[0].id;
    game.players[0]
        .hand
        .push(card(110_701, cards::MOUNTAIN, PlayerId::One));
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 4);

    let ultimate = loyalty_action(&game, wrenn, 2).expect("seven loyalty pays for it");
    game.apply(PlayerId::One, ultimate).expect("it activates");
    settle(&mut game);

    let castable = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell { card, .. } => Some(card),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        !castable.contains(&bears),
        "a creature card is not an instant or sorcery: {castable:?}",
    );
    assert!(
        !castable.contains(&theirs),
        "and their graveyard is not yours: {castable:?}",
    );
}
