//! Minsc & Boo, Timeless Heroes: a hamster every turn, and the hamster is
//! both what the plus grows and what the minus throws.

use super::*;

/// Minsc on the battlefield under Player One since last turn, with a
/// stocked library.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for index in 0..8 {
        game.players[0]
            .library
            .push(card(111_000 + index, cards::ISLAND, PlayerId::One));
    }
    let minsc = game
        .put_onto_battlefield(PlayerId::One, cards::MINSC_BOO_TIMELESS_HEROES)
        .expect("cataloged");
    settle(&mut game, true);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.players[1].life = 20;
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, minsc)
}

/// Answers decisions, taking the first non-declining option when `accept`
/// and declining otherwise.
fn settle(game: &mut Game, accept: bool) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .find(|option| (option.label != "Decline") == accept)
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

fn boos(game: &Game) -> Vec<&Permanent> {
    game.battlefield
        .iter()
        .filter(|permanent| {
            game.effective_permanent_name(permanent)
                .is_some_and(|name| name == "Boo")
        })
        .collect()
}

/// Activates Minsc's printed ability `index`, preferring `wanted` as its
/// target, and lets it resolve.
fn activate(game: &mut Game, minsc: GameObjectId, index: u8, wanted: Option<Target>) {
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
                *source == minsc
                    && *ability == AbilityId(index)
                    && wanted.is_none_or(|wanted| {
                        targets
                            .iter()
                            .any(|selection| selection.targets().contains(&wanted))
                    })
            }
            _ => false,
        })
        .expect("the loyalty ability is offered");
    game.apply(PlayerId::One, action).expect("it is activated");
    settle(game, true);
}

fn loyalty(game: &Game, minsc: GameObjectId) -> u16 {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == minsc)
        .expect("he is there")
        .counters(CounterKind::Loyalty)
}

/// He arrives with Boo.
#[test]
fn he_enters_with_a_hamster() {
    let (game, _) = staged();

    let boo = boos(&game);
    assert_eq!(boo.len(), 1, "one Boo");
    assert_eq!(game.power(boo[0]), Some(1));
    assert_eq!(game.toughness(boo[0]), Some(1));
    assert!(game.permanent_has_executable_keyword(boo[0], KeywordAbility::Trample));
    assert!(game.permanent_has_executable_keyword(boo[0], KeywordAbility::Haste));
}

/// "You may": declining leaves the board as it was.
#[test]
fn the_hamster_may_be_declined() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.put_onto_battlefield(PlayerId::One, cards::MINSC_BOO_TIMELESS_HEROES)
        .expect("cataloged");
    settle(&mut game, false);

    assert!(boos(&game).is_empty(), "no hamster was made");
}

/// The plus puts three counters on a creature with haste -- Boo qualifies.
#[test]
fn the_plus_grows_the_hamster() {
    let (mut game, minsc) = staged();
    let boo = boos(&game)[0].card.id;

    activate(&mut game, minsc, 1, Some(Target::Permanent(boo)));

    let grown = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == boo)
        .expect("Boo is there");
    assert_eq!(game.power(grown), Some(4), "three counters on a 1/1");
    assert_eq!(game.toughness(grown), Some(4));
    assert_eq!(loyalty(&game, minsc), 4);
}

/// A creature with neither trample nor haste is not a legal target.
#[test]
fn a_plain_creature_cannot_be_grown() {
    let (mut game, minsc) = staged();
    let bears = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);

    assert!(
        !game.legal_actions(PlayerId::One).iter().any(|action| {
            matches!(
                action,
                Action::ActivateAbility {
                    source,
                    ability: AbilityOrigin::Printed { ability, .. },
                    targets,
                    ..
                } if *source == minsc
                    && *ability == AbilityId(1)
                    && targets.iter().any(|selection| {
                        selection.targets().contains(&Target::Permanent(bears))
                    })
            )
        }),
        "the bear has neither keyword",
    );
}

/// The minus throws the hamster: X damage for its power, and X cards
/// because it was a Hamster. Only one loyalty ability a turn, so the plus
/// has not grown it first.
#[test]
fn the_minus_throws_boo_and_draws() {
    let (mut game, minsc) = staged();
    let hand = game.players[0].hand.len();

    activate(&mut game, minsc, 2, Some(Target::Player(PlayerId::Two)));

    assert_eq!(game.players[1].life, 19, "one power is one damage");
    assert_eq!(
        game.players[0].hand.len(),
        hand + 1,
        "and a Hamster draws that many cards",
    );
    assert!(boos(&game).is_empty(), "Boo was thrown");
    assert_eq!(loyalty(&game, minsc), 1, "three minus two");
}

/// A grown Boo throws for more: the counters are part of its power when the
/// minus reads it.
#[test]
fn a_grown_hamster_throws_for_more() {
    let (mut game, minsc) = staged();
    let boo = boos(&game)[0].card.id;
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == boo)
        .expect("Boo is there")
        .set_counters(CounterKind::PlusOnePlusOne, 3);
    let hand = game.players[0].hand.len();

    activate(&mut game, minsc, 2, Some(Target::Player(PlayerId::Two)));

    assert_eq!(game.players[1].life, 16, "four power is four damage");
    assert_eq!(game.players[0].hand.len(), hand + 4, "and four cards");
}

/// A creature that is not a Hamster deals its damage and draws nothing.
#[test]
fn throwing_something_else_draws_nothing() {
    let (mut game, minsc) = staged();
    let boo = boos(&game)[0].card.id;
    game.battlefield
        .retain(|permanent| permanent.card.id != boo);
    game.put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    let hand = game.players[0].hand.len();

    activate(&mut game, minsc, 2, Some(Target::Player(PlayerId::Two)));

    assert_eq!(game.players[1].life, 18, "the bear's two power");
    assert_eq!(game.players[0].hand.len(), hand, "and no cards");
}

/// The other half of the same clause: your upkeep offers Boo too. Declined
/// on the way in, so what arrives here is the upkeep's doing.
#[test]
fn your_upkeep_offers_another_hamster() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.put_onto_battlefield(PlayerId::One, cards::MINSC_BOO_TIMELESS_HEROES)
        .expect("cataloged");
    settle(&mut game, false);
    assert!(boos(&game).is_empty(), "the arrival was declined");
    game.active_player = PlayerId::One;
    game.priority = PlayerId::One;

    game.step = Step::Upkeep;
    game.handle_upkeep_triggers();
    settle(&mut game, true);

    assert_eq!(boos(&game).len(), 1, "the upkeep brought him along");
}

/// It is your upkeep and not theirs.
#[test]
fn their_upkeep_offers_nothing() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.put_onto_battlefield(PlayerId::One, cards::MINSC_BOO_TIMELESS_HEROES)
        .expect("cataloged");
    settle(&mut game, false);
    game.active_player = PlayerId::Two;
    game.priority = PlayerId::Two;

    game.step = Step::Upkeep;
    game.handle_upkeep_triggers();
    settle(&mut game, true);

    assert!(boos(&game).is_empty(), "their upkeep is not yours");
}

/// "If there is no legal target for the reflexive trigger ... or if the
/// target is illegal as the ability tries to resolve, you will not draw any
/// cards even if the sacrificed creature was a Hamster." The target is
/// declared here as the minus is activated rather than by a reflexive
/// trigger, so an answer to the target counters the whole ability: no
/// damage and no cards, and -- unlike the printed card, where the sacrifice
/// has already happened by then -- the Hamster is still standing.
#[test]
fn an_answered_target_costs_the_damage_and_the_cards() {
    let (mut game, minsc) = staged();
    let bears = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    let hand = game.players[0].hand.len();

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
                *source == minsc
                    && *ability == AbilityId(2)
                    && targets
                        .iter()
                        .any(|selection| selection.targets().contains(&Target::Permanent(bears)))
            }
            _ => false,
        })
        .expect("the minus can name the Bears");
    game.apply(PlayerId::One, action).expect("it is activated");
    // The Bears are answered while the minus is on the stack.
    game.move_permanents_to_graveyard(&[bears]);
    settle(&mut game, true);

    assert_eq!(
        game.players[0].hand.len(),
        hand,
        "a Hamster draws nothing when the throw has nowhere to land",
    );
    assert_eq!(game.players[1].life, 20, "and nothing took the damage");
    assert_eq!(loyalty(&game, minsc), 1, "the loyalty was still paid");
    assert!(
        !boos(&game).is_empty(),
        "the deviation this card carries: the target is named on activation, \
         so the whole ability is countered and Boo is never thrown",
    );
}
