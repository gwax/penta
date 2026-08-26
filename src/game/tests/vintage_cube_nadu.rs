//! Nadu, Winged Wisdom: every targeting spell you own is a card, twice per
//! creature per turn.

use super::*;

/// Nadu on the battlefield under Player One with `library` on top of their
/// library -- the last is the top card -- and a bear beside him.
fn staged(library: &[CardDefinitionId]) -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for definition in library {
        let card = game
            .build_zone(PlayerId::One, &[*definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        game.players[0].library.push(card);
    }
    let nadu = game
        .put_onto_battlefield(PlayerId::One, cards::NADU_WINGED_WISDOM)
        .expect("cataloged");
    let bears = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, nadu, bears)
}

fn settle(game: &mut Game) {
    for _ in 0..32 {
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
    drain_pending(game);
}

/// Player One points a Giant Growth at `target` and lets it resolve.
fn pump(game: &mut Game, target: GameObjectId) {
    let card = game
        .build_zone(PlayerId::One, &[cards::GIANT_GROWTH])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let held = card.id;
    game.players[0].hand.push(card);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 1);
    let action =
        game.legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| match action {
                Action::CastSpell { card, choices, .. } => {
                    *card == held
                        && choices.targets().iter().any(|selection| {
                            selection.targets().contains(&Target::Permanent(target))
                        })
                }
                _ => false,
            })
            .expect("it can point at that creature");
    game.apply(PlayerId::One, action).expect("it is cast");
    settle(game);
}

/// A land on top goes to the battlefield.
#[test]
fn a_targeted_creature_puts_a_land_onto_the_battlefield() {
    let (mut game, _, bears) = staged(&[cards::ISLAND, cards::FOREST]);

    pump(&mut game, bears);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::FOREST),
        "the land arrived",
    );
    assert!(game.players[0].hand.is_empty(), "and not into the hand");
    assert_eq!(game.players[0].library.len(), 1);
}

/// Anything else goes to the hand.
#[test]
fn a_nonland_goes_to_the_hand() {
    let (mut game, _, bears) = staged(&[cards::ISLAND, cards::LIGHTNING_BOLT]);

    pump(&mut game, bears);

    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::LIGHTNING_BOLT),
        "the Bolt is in hand",
    );
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::LIGHTNING_BOLT),
    );
}

/// Nadu is a creature you control, so pointing something at him counts too.
#[test]
fn nadu_triggers_off_himself() {
    let (mut game, nadu, _) = staged(&[cards::ISLAND, cards::FOREST]);

    pump(&mut game, nadu);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::FOREST),
        "he has the granted ability like everything else",
    );
}

/// "Only twice each turn": the third spell aimed at the same creature does
/// nothing, and a different creature has its own two.
#[test]
fn each_creature_gets_two_a_turn() {
    let (mut game, nadu, bears) = staged(&[
        cards::ISLAND,
        cards::LIGHTNING_BOLT,
        cards::LIGHTNING_BOLT,
        cards::LIGHTNING_BOLT,
        cards::LIGHTNING_BOLT,
    ]);

    pump(&mut game, bears);
    pump(&mut game, bears);
    assert_eq!(game.players[0].hand.len(), 2, "two off the bear");

    pump(&mut game, bears);
    assert_eq!(game.players[0].hand.len(), 2, "and the third gives nothing");

    pump(&mut game, nadu);
    assert_eq!(
        game.players[0].hand.len(),
        3,
        "but Nadu's own copy has not been spent",
    );
}

/// The cap is per turn: a new turn is two more.
#[test]
fn the_cap_resets_with_the_turn() {
    let (mut game, _, bears) = staged(&[
        cards::ISLAND,
        cards::LIGHTNING_BOLT,
        cards::LIGHTNING_BOLT,
        cards::LIGHTNING_BOLT,
    ]);
    pump(&mut game, bears);
    pump(&mut game, bears);
    pump(&mut game, bears);
    assert_eq!(game.players[0].hand.len(), 2);

    let turn = game.turn;
    for _ in 0..80 {
        if game.turn > turn + 1 {
            break;
        }
        game.advance_step();
        settle(&mut game);
    }
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    let before = game.players[0].hand.len();

    pump(&mut game, bears);

    assert_eq!(
        game.players[0].hand.len(),
        before + 1,
        "the count is a per-turn one",
    );
}

/// A creature an opponent controls has no such ability.
#[test]
fn their_creature_gives_them_nothing() {
    let (mut game, _, _) = staged(&[cards::ISLAND, cards::FOREST]);
    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::SAVANNAH_LIONS)
        .expect("cataloged");
    drain_pending(&mut game);

    pump(&mut game, theirs);

    assert!(
        game.players[0].hand.is_empty(),
        "their creature carries nothing of Nadu's",
    );
    assert_eq!(game.players[0].library.len(), 2, "the library is untouched");
}
