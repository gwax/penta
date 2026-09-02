//! Paradoxical Outcome: pick up as many of your own permanents as you like,
//! and draw for the ones that came back to your hand.

use super::*;

/// Player One with an Outcome in hand and the mana to cast it.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let outcome = game
        .build_zone(PlayerId::One, &[cards::PARADOXICAL_OUTCOME])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = outcome.id;
    game.players[0].hand.push(outcome);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 4);
    game.priority = PlayerId::One;
    (game, id)
}

fn resolve(game: &mut Game) {
    for _ in 0..16 {
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

/// Casts the Outcome naming exactly `wanted`, and resolves it.
fn outcome(game: &mut Game, spell: GameObjectId, wanted: &[GameObjectId]) {
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                if *card != spell {
                    return false;
                }
                let named = choices.iter_targets().copied().collect::<Vec<_>>();
                named.len() == wanted.len()
                    && wanted
                        .iter()
                        .all(|id| named.contains(&Target::Permanent(*id)))
            }
            _ => false,
        })
        .expect("that combination of targets is on offer");
    game.apply(PlayerId::One, cast).expect("it is castable");
    resolve(game);
}

fn on_battlefield(game: &Game, id: GameObjectId) -> bool {
    game.battlefield
        .iter()
        .any(|permanent| permanent.card.id == id)
}

/// Two picked up is two drawn.
#[test]
fn it_draws_for_each_permanent_returned() {
    let (mut game, spell) = staged();
    let first = game
        .put_onto_battlefield(PlayerId::One, cards::MOX_SAPPHIRE)
        .expect("cataloged");
    let second = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    outcome(&mut game, spell, &[first, second]);

    assert!(!on_battlefield(&game, first) && !on_battlefield(&game, second));
    assert_eq!(game.players[0].hand.len(), 4, "two picked up and two drawn");
}

/// Naming none is legal, and draws none.
#[test]
fn naming_nothing_draws_nothing() {
    let (mut game, spell) = staged();
    game.put_onto_battlefield(PlayerId::One, cards::MOX_SAPPHIRE)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    outcome(&mut game, spell, &[]);

    assert!(
        game.players[0].hand.is_empty(),
        "nothing back and nothing drawn"
    );
    assert_eq!(game.battlefield.len(), 1, "and the Mox stayed put");
}

/// A land and a token are not legal targets, so an Outcome with only those
/// on the battlefield can name nothing at all.
#[test]
fn lands_and_tokens_are_not_legal_targets() {
    let (mut game, spell) = staged();
    game.put_onto_battlefield(PlayerId::One, cards::MOUNTAIN)
        .expect("cataloged");
    game.create_token(
        PlayerId::One,
        token_with_flying(tokens::creature(&["Spirit"], &[ManaColor::White], 1, 1)),
    );
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    let named = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell { card, choices, .. } if card == spell => {
                Some(choices.iter_targets().count())
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(named, vec![0], "the only cast names nothing");
}

/// A permanent you control but do not own goes back to its owner's hand, so
/// it returns without paying you a card.
#[test]
fn a_permanent_you_do_not_own_draws_you_nothing() {
    let (mut game, spell) = staged();
    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::MOX_SAPPHIRE)
        .expect("cataloged");
    drain_pending(&mut game);
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == theirs)
    {
        permanent.controller = PlayerId::One;
    }
    game.priority = PlayerId::One;

    outcome(&mut game, spell, &[theirs]);

    assert!(!on_battlefield(&game, theirs), "it left the battlefield");
    assert!(
        game.players[1]
            .hand
            .iter()
            .any(|card| card.definition == cards::MOX_SAPPHIRE),
        "and went to its owner's hand",
    );
    assert!(
        game.players[0].hand.is_empty(),
        "which draws you nothing at all",
    );
}

/// "Permanents you control": theirs are never on the list, however
/// nonland and nontoken they are.
#[test]
fn their_permanents_are_not_on_the_list() {
    let (mut game, spell) = staged();
    let mine = game
        .put_onto_battlefield(PlayerId::One, cards::MOX_JET)
        .expect("cataloged");
    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::MOX_RUBY)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    let named = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell { card, choices, .. } if card == spell => Some(
                choices
                    .iter_targets()
                    .filter_map(|target| match target {
                        Target::Permanent(id) => Some(*id),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .flatten()
        .collect::<Vec<_>>();

    assert!(named.contains(&mine), "your own Mox is a target");
    assert!(!named.contains(&theirs), "and theirs is not: {named:?}");
}

/// The targets are named as it is cast: one answered in response is simply
/// not returned, and the draw shrinks with it while the rest come back.
#[test]
fn a_target_answered_in_response_costs_its_own_card() {
    let (mut game, spell) = staged();
    let doomed = game
        .put_onto_battlefield(PlayerId::One, cards::MOX_JET)
        .expect("cataloged");
    let survivor = game
        .put_onto_battlefield(PlayerId::One, cards::MOX_RUBY)
        .expect("cataloged");
    drain_pending(&mut game);
    game.players[PlayerId::One.index()]
        .hand
        .retain(|card| card.definition == cards::PARADOXICAL_OUTCOME);
    game.priority = PlayerId::One;
    let library = game.players[PlayerId::One.index()].library.len();

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                let named = choices.iter_targets().copied().collect::<Vec<_>>();
                *card == spell
                    && named.len() == 2
                    && named.contains(&Target::Permanent(doomed))
                    && named.contains(&Target::Permanent(survivor))
            }
            _ => false,
        })
        .expect("both Moxen are on offer together");
    game.apply(PlayerId::One, cast).expect("it is castable");

    // In response, one of the two is gone.
    game.move_permanents_to_graveyard(&[doomed]);
    resolve(&mut game);

    assert!(
        !on_battlefield(&game, survivor),
        "the Mox that was still there came back",
    );
    assert_eq!(
        game.players[PlayerId::One.index()]
            .hand
            .iter()
            .filter(|card| card.definition == cards::MOX_RUBY)
            .count(),
        1,
        "into hand",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].library.len(),
        library - 1,
        "and one card drawn: the dead Mox pays nothing",
    );
}

/// It is an instant: their end step is as good a window as your own main
/// phase, which is how the rocks are picked up with their spell waiting.
#[test]
fn it_may_be_cast_on_their_turn() {
    let (mut game, spell) = staged();
    let lotus = game
        .put_onto_battlefield(PlayerId::One, cards::BLACK_LOTUS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.active_player = PlayerId::Two;
    game.step = Step::End;
    game.priority = PlayerId::One;

    outcome(&mut game, spell, &[lotus]);

    assert!(!on_battlefield(&game, lotus), "the Lotus came back");
    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::BLACK_LOTUS),
        "into your hand, on their turn",
    );
}

/// Returning a permanent is a permanent leaving, so what watches for that
/// fires: a Tidehollow Sculler picked up hands back the card it was holding.
#[test]
fn what_it_returns_leaves_the_battlefield_for_every_purpose() {
    let (mut game, spell) = staged();
    game.players[1].hand.clear();
    game.players[1]
        .hand
        .push(card(101_900, cards::LIGHTNING_BOLT, PlayerId::Two));
    let sculler = game
        .put_onto_battlefield(PlayerId::One, cards::TIDEHOLLOW_SCULLER)
        .expect("cataloged");
    for _ in 0..12 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options: vec![decision.options[0].id],
                },
            )
            .expect("the offered choice is legal");
            continue;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        if game.apply(game.priority, Action::PassPriority).is_err() {
            break;
        }
    }
    game.priority = PlayerId::One;
    assert_eq!(
        game.players[1].exile.len(),
        1,
        "the Sculler is holding their Bolt",
    );

    outcome(&mut game, spell, &[sculler]);

    assert!(!on_battlefield(&game, sculler), "the Sculler came back");
    assert!(
        game.players[1]
            .hand
            .iter()
            .any(|card| card.definition == cards::LIGHTNING_BOLT),
        "and its leave trigger gave the Bolt back",
    );
    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::TIDEHOLLOW_SCULLER),
        "while the Sculler itself is a card you drew nothing for beyond its own",
    );
}
