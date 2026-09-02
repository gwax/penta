//! Inquisition of Kozilek: one mana, no life, and everything the format
//! actually casts on the first three turns.

use super::*;

/// Player One holding an Inquisition, Player Two holding `hand`.
fn staged(hand: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[1].hand.clear();
    for definition in hand {
        let card = game
            .build_zone(PlayerId::Two, &[*definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        game.players[1].hand.push(card);
    }
    let inquisition = game
        .build_zone(PlayerId::One, &[cards::INQUISITION_OF_KOZILEK])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = inquisition.id;
    game.players[0].hand.push(inquisition);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    game.priority = PlayerId::One;
    (game, id)
}

fn cast_at_two(game: &mut Game, spell: GameObjectId) {
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == spell
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Player(PlayerId::Two))
            }
            _ => false,
        })
        .expect("one black mana aimed across the table");
    game.apply(PlayerId::One, cast).expect("it is castable");
}

/// Casts it and answers whatever it asks, taking `wanted` when offered.
fn inquire(game: &mut Game, spell: GameObjectId, wanted: Option<CardDefinitionId>) {
    cast_at_two(game, spell);
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
                .or_else(|| decision.options.first())
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
            return;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            return;
        }
    }
}

/// The choice it is waiting on, once the spell is on the stack.
fn offered(game: &mut Game, spell: GameObjectId) -> Vec<CardDefinitionId> {
    cast_at_two(game, spell);
    for _ in 0..8 {
        if !game.pending_decisions.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
    game.pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .into_iter()
        .flat_map(|decision| decision.options)
        .filter_map(|option| option.card.map(|(_, characteristics)| characteristics))
        .filter_map(ObjectCharacteristics::card_definition)
        .collect()
}

fn in_graveyard(game: &Game, definition: CardDefinitionId) -> bool {
    game.players[1]
        .graveyard
        .iter()
        .any(|card| card.definition == definition)
}

/// It takes the card you choose, and only that one.
#[test]
fn it_takes_the_card_you_choose() {
    let (mut game, spell) = staged(&[cards::GRIZZLY_BEARS, cards::LIGHTNING_BOLT]);

    inquire(&mut game, spell, Some(cards::LIGHTNING_BOLT));

    assert!(in_graveyard(&game, cards::LIGHTNING_BOLT), "the one chosen");
    assert!(
        !in_graveyard(&game, cards::GRIZZLY_BEARS),
        "and only the one chosen",
    );
    assert_eq!(game.players[1].life, 20, "and it costs them no life");
    assert_eq!(game.players[0].life, 20, "and you none either");
}

/// Three is the bound, and it includes three. Two cheap cards, so the
/// choice is a real one and what it offers is a list to look at.
#[test]
fn it_reaches_three_and_no_further() {
    let (mut game, spell) = staged(&[
        cards::STONE_RAIN,
        cards::LIGHTNING_BOLT,
        cards::ICY_MANIPULATOR,
        cards::SHIVAN_DRAGON,
    ]);

    let mut candidates = offered(&mut game, spell);
    candidates.sort_by_key(|definition| format!("{definition:?}"));
    let mut expected = vec![cards::STONE_RAIN, cards::LIGHTNING_BOLT];
    expected.sort_by_key(|definition| format!("{definition:?}"));
    assert_eq!(
        candidates, expected,
        "a three-mana sorcery and a one-mana instant, and nothing dearer",
    );
}

/// A land is not a legal choice however cheap it is.
#[test]
fn a_land_is_not_a_legal_choice() {
    let (mut game, spell) = staged(&[cards::MOUNTAIN, cards::GRIZZLY_BEARS, cards::LIGHTNING_BOLT]);

    let candidates = offered(&mut game, spell);
    assert_eq!(candidates.len(), 2, "both nonland cards and no more");
    assert!(
        !candidates.contains(&cards::MOUNTAIN),
        "the land is safe: {candidates:?}",
    );
}

/// A hand it cannot touch loses nothing, and the spell still resolves.
#[test]
fn an_expensive_hand_loses_nothing() {
    let (mut game, spell) = staged(&[cards::SHIVAN_DRAGON, cards::MOUNTAIN]);

    inquire(&mut game, spell, None);

    assert_eq!(game.players[1].hand.len(), 2, "nothing was taken");
    assert!(game.players[1].graveyard.is_empty());
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::INQUISITION_OF_KOZILEK),
        "and the Inquisition resolved anyway",
    );
}

/// Their hand is revealed, not looked at: it is public information.
#[test]
fn their_hand_is_revealed() {
    let (mut game, spell) = staged(&[cards::GRIZZLY_BEARS, cards::LIGHTNING_BOLT]);
    let before = game.events.len();

    inquire(&mut game, spell, Some(cards::GRIZZLY_BEARS));

    let revealed = game.events[before..]
        .iter()
        .filter(|event| {
            matches!(
                event,
                GameEvent::CardRevealed {
                    player: PlayerId::Two,
                    ..
                }
            )
        })
        .count();
    assert_eq!(revealed, 2, "both cards were shown, not just the one taken");
}

/// "If you target yourself with this spell, you must reveal your entire hand
/// to the other players just as any other player would." Pointing it at
/// yourself is legal, and it is a reveal rather than a look either way.
#[test]
fn aimed_at_yourself_it_reveals_your_own_hand() {
    let (mut game, spell) = staged(&[]);
    for definition in [cards::GRIZZLY_BEARS, cards::LIGHTNING_BOLT] {
        let card = game
            .build_zone(PlayerId::One, &[definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        game.players[0].hand.push(card);
    }
    let before = game.events.len();

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == spell
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Player(PlayerId::One))
            }
            _ => false,
        })
        .expect("you are a legal target for your own Inquisition");
    game.apply(PlayerId::One, cast).expect("it is castable");
    for _ in 0..16 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .find(|option| {
                    matches!(
                        option.card,
                        Some((_, ObjectCharacteristics::Card { definition, .. }))
                            if definition == cards::GRIZZLY_BEARS
                    )
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

    let revealed = game.events[before..]
        .iter()
        .filter(|event| {
            matches!(
                event,
                GameEvent::CardRevealed {
                    player: PlayerId::One,
                    ..
                }
            )
        })
        .count();
    assert_eq!(revealed, 2, "your whole hand is shown, both cards of it");
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS),
        "and you discard what you chose out of your own hand",
    );
}

/// X counts as zero anywhere but the stack (CR 202.3b), so a Fireball in
/// hand is worth one and a Walking Ballista nothing at all: the two cards
/// that would cost the most on the way down are the two the Inquisition
/// takes most easily. A Serra Angel beside them is still out of reach.
#[test]
fn an_x_spell_in_hand_is_as_cheap_as_its_pips() {
    let (mut game, inquisition) =
        staged(&[cards::FIREBALL, cards::WALKING_BALLISTA, cards::SERRA_ANGEL]);

    let mut choices = offered(&mut game, inquisition);
    choices.sort_unstable();
    let mut expected = vec![cards::FIREBALL, cards::WALKING_BALLISTA];
    expected.sort_unstable();
    assert_eq!(
        choices, expected,
        "both X spells are on offer and the five-drop is not",
    );
}

/// And taking one is taking it: the Fireball goes to the graveyard like any
/// other three-or-less card.
#[test]
fn the_x_spell_it_takes_is_discarded() {
    let (mut game, inquisition) = staged(&[cards::FIREBALL, cards::SERRA_ANGEL]);

    inquire(&mut game, inquisition, Some(cards::FIREBALL));

    assert!(in_graveyard(&game, cards::FIREBALL), "the Fireball went");
    assert!(
        game.players[1]
            .hand
            .iter()
            .any(|card| card.definition == cards::SERRA_ANGEL),
        "and the Angel stayed where it was",
    );
}
