//! Galvanic Discharge: one mana kills a three-toughness creature, and the
//! energy it does not spend stays banked.

use super::*;

/// Player One holding the Discharge with a red mana up and `energy` already
/// banked, with `theirs` on the battlefield opposite.
fn staged(theirs: CardDefinitionId, energy: u16) -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let target = game
        .put_onto_battlefield(PlayerId::Two, theirs)
        .expect("cataloged");
    drain_pending(&mut game);
    let card = game
        .build_zone(PlayerId::One, &[cards::GALVANIC_DISCHARGE])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let held = card.id;
    game.players[0].hand.push(card);
    game.players[0].counters.set(CounterKind::Energy, energy);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, held, target)
}

fn energy(game: &Game) -> u16 {
    game.players[0].counters.count(CounterKind::Energy)
}

fn alive(game: &Game, id: GameObjectId) -> bool {
    game.battlefield
        .iter()
        .any(|permanent| permanent.card.id == id)
}

/// Casts the Discharge at `target` and pays `pay` energy, or declines when
/// `pay` is zero.
fn discharge(game: &mut Game, held: GameObjectId, target: GameObjectId, pay: u32) {
    let cast =
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
            .expect("it can point at that permanent");
    game.apply(PlayerId::One, cast).expect("it is cast");
    for _ in 0..16 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            // The option ids are the amounts, so paying is a matter of
            // naming how much; zero is the declining option.
            let options = decision
                .options
                .iter()
                .find(|option| option.id == pay)
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
    drain_pending(game);
}

/// Three energy is three damage, which is what the card is played for.
#[test]
fn three_energy_kills_a_three_toughness_creature() {
    let (mut game, held, bears) = staged(cards::GRIZZLY_BEARS, 0);

    discharge(&mut game, held, bears, 3);

    assert!(!alive(&game, bears), "two toughness is well inside three");
    assert_eq!(energy(&game), 0, "all three were spent");
}

/// The energy it does not need stays banked.
#[test]
fn the_leftover_energy_stays() {
    let (mut game, held, bears) = staged(cards::GRIZZLY_BEARS, 0);

    discharge(&mut game, held, bears, 2);

    assert!(!alive(&game, bears), "two damage is enough for a 2/2");
    assert_eq!(energy(&game), 1, "and the third counter is still there");
}

/// Energy banked before it is cast is energy it may spend.
#[test]
fn banked_energy_makes_it_bigger() {
    let (mut game, held, titan) = staged(cards::GRAVE_TITAN, 3);

    discharge(&mut game, held, titan, 6);

    assert!(!alive(&game, titan), "six damage kills a 6/6");
    assert_eq!(energy(&game), 0);
}

/// "You may pay": paying nothing is a legal answer, and the three counters
/// are still yours.
#[test]
fn paying_nothing_still_banks_the_energy() {
    let (mut game, held, bears) = staged(cards::GRIZZLY_BEARS, 0);

    discharge(&mut game, held, bears, 0);

    assert!(alive(&game, bears), "no damage was dealt");
    assert_eq!(energy(&game), 3, "and the three counters are yours");
}

/// It answers a planeswalker as readily as a creature.
#[test]
fn it_answers_a_planeswalker() {
    let (mut game, held, teferi) = staged(cards::TEFERI_HERO_OF_DOMINARIA, 1);

    discharge(&mut game, held, teferi, 4);

    assert!(!alive(&game, teferi), "four loyalty is all he had");
}
