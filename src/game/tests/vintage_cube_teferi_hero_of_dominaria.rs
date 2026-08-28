//! Teferi, Hero of Dominaria: a card and two lands back every turn, an
//! answer that buries a permanent two draws deep, and an emblem that turns
//! every later draw into exile.

use super::*;

/// Teferi on the battlefield with `loyalty` counters, `lands` tapped lands
/// under Player One, and a stocked library.
fn staged(loyalty: u16, lands: usize) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for index in 0..6 {
        game.players[0]
            .library
            .push(card(94_000 + index, cards::ISLAND, PlayerId::One));
    }
    let teferi = game
        .put_onto_battlefield(PlayerId::One, cards::TEFERI_HERO_OF_DOMINARIA)
        .expect("cataloged");
    for _ in 0..lands {
        game.put_onto_battlefield(PlayerId::One, cards::ISLAND)
            .expect("cataloged");
    }
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = permanent.card.definition == cards::ISLAND;
    }
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == teferi)
        .expect("he is there")
        .set_counters(CounterKind::Loyalty, loyalty);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, teferi)
}

/// Answers pending decisions, preferring options that name one of `wanted`.
fn settle_choosing(game: &mut Game, wanted: &[GameObjectId]) {
    for _ in 0..16 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let mut options = decision
                .options
                .iter()
                .filter(|option| {
                    option
                        .card
                        .is_some_and(|(object, _)| wanted.contains(&object))
                })
                .map(|option| option.id)
                .collect::<Vec<_>>();
            if options.is_empty() {
                options = decision
                    .options
                    .iter()
                    .map(|option| option.id)
                    .take(decision.minimum)
                    .collect();
            }
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

/// Activates Teferi's printed ability `index`, aimed at `wanted` where the
/// ability takes targets, and lets it resolve.
fn activate(game: &mut Game, teferi: GameObjectId, index: u8, wanted: &[GameObjectId]) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source,
                ability: AbilityOrigin::Printed { ability, .. },
                targets,
                ..
            } => {
                *source == teferi
                    && *ability == AbilityId(index)
                    && (wanted.is_empty() || aimed_at(targets, wanted))
            }
            _ => false,
        })
        .expect("the loyalty ability is offered");
    game.apply(PlayerId::One, action).expect("it is activated");
    settle_choosing(game, wanted);
}

/// Whether every chosen target is one of `wanted`.
fn aimed_at(targets: &[TargetSelection], wanted: &[GameObjectId]) -> bool {
    !targets.is_empty()
        && targets.iter().all(|selection| {
            selection.targets().iter().all(|target| {
                matches!(target, Target::Permanent(id) | Target::Card(id) if wanted.contains(id))
            })
        })
}

/// Passes priority until somebody is asked something, which is where a
/// trigger's own targets and choices are named.
fn advance_to_decision(game: &mut Game) {
    for _ in 0..12 {
        if !game.pending_decisions.is_empty() {
            return;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
}

fn end_step(game: &mut Game, wanted: &[GameObjectId]) {
    game.step = Step::End;
    game.begin_step_triggers();
    settle_choosing(game, wanted);
}

fn tapped_lands(game: &Game) -> usize {
    game.battlefield
        .iter()
        .filter(|permanent| permanent.card.definition == cards::ISLAND && permanent.tapped)
        .count()
}

/// The card comes now; the lands come back at the end step, not before.
#[test]
fn the_plus_draws_now_and_untaps_later() {
    let (mut game, teferi) = staged(4, 3);
    let hand = game.players[0].hand.len();

    activate(&mut game, teferi, 0, &[]);

    assert_eq!(game.players[0].hand.len(), hand + 1, "the card arrives now");
    assert_eq!(tapped_lands(&game), 3, "and the lands are still down");

    let lands = game
        .battlefield
        .iter()
        .filter(|permanent| permanent.card.definition == cards::ISLAND)
        .map(|permanent| permanent.card.id)
        .take(2)
        .collect::<Vec<_>>();
    end_step(&mut game, &lands);

    assert_eq!(tapped_lands(&game), 1, "two of the three came back up");
}

/// It is the *next* end step and only that one: the delayed trigger does not
/// come back on a later turn.
#[test]
fn the_untap_happens_once() {
    let (mut game, teferi) = staged(4, 3);
    activate(&mut game, teferi, 0, &[]);
    let lands = game
        .battlefield
        .iter()
        .filter(|permanent| permanent.card.definition == cards::ISLAND)
        .map(|permanent| permanent.card.id)
        .take(2)
        .collect::<Vec<_>>();
    end_step(&mut game, &lands);
    for permanent in &mut game.battlefield {
        permanent.tapped = permanent.card.definition == cards::ISLAND;
    }

    game.turns_started = [6, 5];
    end_step(&mut game, &lands);

    assert_eq!(
        tapped_lands(&game),
        3,
        "the trigger was spent the first time"
    );
}

/// "Up to two" is a maximum, not a requirement, and the lands are anyone's:
/// the choice offers the other player's lands too.
#[test]
fn the_untap_may_take_any_lands() {
    let (mut game, teferi) = staged(4, 1);
    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::MOUNTAIN)
        .expect("cataloged");
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == theirs)
        .expect("it is there")
        .tapped = true;
    activate(&mut game, teferi, 0, &[]);

    game.step = Step::End;
    game.begin_step_triggers();
    advance_to_decision(&mut game);
    let offered = game
        .pending_decisions
        .first()
        .expect("the delayed trigger is asking")
        .observation
        .options
        .iter()
        .filter_map(|option| option.card.map(|(object, _)| object))
        .collect::<Vec<_>>();
    assert!(
        offered.contains(&theirs),
        "nothing in the clause says whose lands they are",
    );

    settle_choosing(&mut game, &[theirs]);
    assert!(
        !game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == theirs)
            .expect("still there")
            .tapped,
        "and untapping theirs is a legal way to spend it",
    );
}

/// Third from the top means two cards above it, which is two draws away.
#[test]
fn the_minus_three_buries_it_two_deep() {
    let (mut game, teferi) = staged(4, 0);
    let bear = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    let library = game.players[1].library.len();

    activate(&mut game, teferi, 1, &[bear]);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == bear),
        "it left the battlefield",
    );
    let owner = &game.players[1].library;
    assert_eq!(owner.len(), library + 1, "into its owner's library");
    assert_eq!(
        owner
            .iter()
            .rev()
            .position(|card| card.definition == cards::GRIZZLY_BEARS),
        Some(2),
        "with exactly two cards above it",
    );
}

/// A library too short to have a third card from the top takes it on the
/// bottom instead, which is where counting down runs out.
#[test]
fn a_short_library_takes_it_on_the_bottom() {
    let (mut game, teferi) = staged(4, 0);
    let bear = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.players[1].library.clear();
    game.players[1]
        .library
        .push(card(95_000, cards::MOUNTAIN, PlayerId::Two));

    activate(&mut game, teferi, 1, &[bear]);

    assert_eq!(
        game.players[1].library[0].definition,
        cards::GRIZZLY_BEARS,
        "under the one card there was",
    );
}

/// The emblem turns every later draw into exile, one permanent per card.
#[test]
fn the_emblem_exiles_on_every_draw() {
    let (mut game, teferi) = staged(8, 0);
    let bear = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);

    activate(&mut game, teferi, 2, &[]);
    assert_eq!(game.emblems.len(), 1, "the emblem is in the command zone");

    game.draw_card(PlayerId::One);
    settle_choosing(&mut game, &[bear]);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == bear),
        "the draw exiled their creature",
    );
}

/// Their permanents only: the emblem cannot be pointed at your own board.
#[test]
fn the_emblem_reads_an_opponents_permanent() {
    let (mut game, teferi) = staged(8, 1);
    let bear = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    activate(&mut game, teferi, 2, &[]);

    game.draw_card(PlayerId::One);
    advance_to_decision(&mut game);
    let offered = game
        .pending_decisions
        .first()
        .expect("the emblem is asking")
        .observation
        .options
        .iter()
        .filter_map(|option| option.card.map(|(object, _)| object))
        .collect::<Vec<_>>();

    assert_eq!(offered, vec![bear], "only their side is a legal target");
}

/// "You choose the target for the triggered ability of Teferi's emblem
/// after you've seen the card you drew." The draw is what triggers it, so
/// the card is in hand by the time the question is asked -- which is what
/// lets a drawn answer decide which permanent to eat.
#[test]
fn the_emblem_asks_after_the_card_is_in_hand() {
    let (mut game, teferi) = staged(8, 0);
    let bear = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    activate(&mut game, teferi, 2, &[]);
    let hand = game.players[0].hand.len();
    let library = game.players[0].library.len();

    game.draw_card(PlayerId::One);
    advance_to_decision(&mut game);

    assert!(
        game.pending_decisions.first().is_some_and(|pending| pending
            .observation
            .options
            .iter()
            .any(|option| option.card.map(|(object, _)| object) == Some(bear))),
        "the emblem is asking which permanent to exile",
    );
    assert_eq!(
        game.players[0].hand.len(),
        hand + 1,
        "and the card it drew is already in hand",
    );
    assert_eq!(game.players[0].library.len(), library - 1);
}

/// A token put into a library ceases to exist (CR 111.7), so the minus
/// answers one for good rather than burying it two cards deep.
#[test]
fn a_tucked_token_simply_stops_existing() {
    let (mut game, teferi) = staged(4, 0);
    game.put_onto_battlefield(PlayerId::Two, cards::ESIKA_S_CHARIOT)
        .expect("cataloged");
    drain_pending(&mut game);
    let cat = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == ObjectKind::Token)
        .expect("the Chariot brought its Cats")
        .card
        .id;
    let library = game.players[1].library.len();

    activate(&mut game, teferi, 1, &[cat]);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == cat),
        "the Cat left the battlefield",
    );
    assert_eq!(
        game.players[1].library.len(),
        library,
        "and no card joined their library: a token in any other zone is gone",
    );
}
