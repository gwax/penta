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
    game.players[0]
        .counters
        .set(CounterKind::named("energy"), energy);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, held, target)
}

fn energy(game: &Game) -> u16 {
    game.players[0].counters.count(CounterKind::named("energy"))
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

/// "If a spell that states you 'may pay' some amount of {E} has become an
/// illegal target, the spell won't resolve. You can't pay any {E} even if
/// you want to." The energy comes with the resolution, so answering the
/// creature costs the caster the counters as well as the damage.
#[test]
fn an_answered_target_costs_the_energy_too() {
    let (mut game, held, target) = staged(cards::GRIZZLY_BEARS, 1);

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
            .expect("it can point at their creature");
    game.apply(PlayerId::One, cast).expect("it is cast");
    game.move_permanents_to_graveyard(&[target]);
    drain_pending(&mut game);
    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    game.check_state_based_actions();

    assert_eq!(
        energy(&game),
        1,
        "the one that was banked is all there is: the spell never resolved",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::GALVANIC_DISCHARGE),
        "and the Discharge was spent for nothing",
    );
}

/// "Energy counters aren't mana. They don't go away as steps, phases, and
/// turns end." What the Discharge banks is there next turn.
#[test]
fn the_energy_survives_the_turn() {
    let (mut game, held, target) = staged(cards::SERRA_ANGEL, 0);
    discharge(&mut game, held, target, 0);
    assert_eq!(energy(&game), 3, "three banked and none spent");

    game.empty_mana_pools();
    game.cleanup();
    game.check_state_based_actions();

    assert_eq!(
        energy(&game),
        3,
        "the cleanup takes mana and damage, not energy",
    );
}

/// "You can't pay more energy counters than you have." The bank at the
/// moment it asks is three plus whatever was already there, and the
/// amounts it offers stop at exactly that.
#[test]
fn it_offers_every_amount_up_to_the_bank_and_no_more() {
    for banked in [0, 2] {
        let (mut game, held, bears) = staged(cards::GRIZZLY_BEARS, banked);
        let cast = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| match action {
                Action::CastSpell { card, choices, .. } => {
                    *card == held
                        && choices.targets().iter().any(|selection| {
                            selection.targets().contains(&Target::Permanent(bears))
                        })
                }
                _ => false,
            })
            .expect("it can point at the Bears");
        game.apply(PlayerId::One, cast).expect("it is cast");
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
            .expect("it asks how much to pay");
        let mut offered = decision
            .options
            .iter()
            .map(|option| option.id)
            .collect::<Vec<_>>();
        offered.sort_unstable();

        assert_eq!(
            offered,
            (0..=u32::from(banked) + 3).collect::<Vec<_>>(),
            "three from the spell and {banked} banked, and nothing above it",
        );
    }
}
