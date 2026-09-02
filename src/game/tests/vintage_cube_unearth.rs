//! Unearth: one black mana for a small creature you already paid for, and a
//! cycling cost for the games where the graveyard has nothing worth raising.

use super::*;

/// Unearth in hand with `graveyard` under player one's graveyard, three
/// black mana up, and a library to cycle into.
fn staged(graveyard: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.turns_started[PlayerId::One.index()] = 5;
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::One.index()].graveyard.clear();
    game.players[PlayerId::One.index()].library.clear();
    game.players[PlayerId::One.index()].library.push(card(
        85_000,
        cards::GIANT_GROWTH,
        PlayerId::One,
    ));
    for (index, definition) in graveyard.iter().enumerate() {
        game.players[PlayerId::One.index()].graveyard.push(card(
            85_100 + u32::try_from(index).expect("few cards"),
            *definition,
            PlayerId::One,
        ));
    }
    let unearth = card(85_200, cards::UNEARTH, PlayerId::One);
    let unearth_id = unearth.id;
    game.players[PlayerId::One.index()].hand.push(unearth);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 3);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, unearth_id)
}

/// The cast that names `target`, if one is offered.
fn cast_at(game: &Game, unearth: GameObjectId, target: GameObjectId) -> Option<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == unearth
                    && choices
                        .targets()
                        .iter()
                        .flat_map(crate::casting::TargetSelection::targets)
                        .any(|chosen| *chosen == Target::Card(target))
            }
            _ => false,
        })
}

fn graveyard_card(game: &Game, definition: CardDefinitionId) -> GameObjectId {
    game.players[PlayerId::One.index()]
        .graveyard
        .iter()
        .find(|card| card.definition == definition)
        .expect("it is in the graveyard")
        .id
}

fn resolve(game: &mut Game) {
    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
}

/// A two-mana creature comes back onto the battlefield.
#[test]
fn it_returns_a_small_creature() {
    let (mut game, unearth) = staged(&[cards::GRIZZLY_BEARS]);
    let bears = graveyard_card(&game, cards::GRIZZLY_BEARS);

    let cast = cast_at(&game, unearth, bears).expect("a 2/2 is small enough");
    game.apply(PlayerId::One, cast).expect("it is cast");
    resolve(&mut game);

    assert!(
        game.battlefield.iter().any(
            |permanent| permanent.card.definition == cards::GRIZZLY_BEARS
                && permanent.controller == PlayerId::One
        ),
        "it comes back under your own control",
    );
    assert_eq!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::UNEARTH],
        "the creature left the graveyard and the sorcery took its place",
    );
}

/// Mana value four is one too many.
#[test]
fn it_will_not_reach_a_big_creature() {
    let (game, unearth) = staged(&[cards::SERRA_ANGEL]);
    let angel = graveyard_card(&game, cards::SERRA_ANGEL);

    assert!(
        cast_at(&game, unearth, angel).is_none(),
        "a five-mana Angel is past the bound",
    );
}

/// Exactly three is inside the bound, not outside it.
#[test]
fn three_is_small_enough() {
    let (mut game, unearth) = staged(&[cards::PHYREXIAN_ARENA, cards::HYPNOTIC_SPECTER]);
    let specter = graveyard_card(&game, cards::HYPNOTIC_SPECTER);
    assert!(cast_at(&game, unearth, specter).is_some());

    let arena = graveyard_card(&game, cards::PHYREXIAN_ARENA);
    assert!(
        cast_at(&game, unearth, arena).is_none(),
        "an enchantment is not a creature card whatever it costs",
    );

    let cast = cast_at(&game, unearth, specter).expect("offered above");
    game.apply(PlayerId::One, cast).expect("it is cast");
    resolve(&mut game);
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::HYPNOTIC_SPECTER),
    );
}

/// With nothing worth raising, it cycles instead.
#[test]
fn it_cycles_when_the_graveyard_is_empty() {
    let (mut game, unearth) = staged(&[]);
    let hand_before = game.players[PlayerId::One.index()].hand.len();

    let cycle = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == unearth),
        )
        .expect("cycling is offered from hand");
    game.apply(PlayerId::One, cycle).expect("it activates");
    resolve(&mut game);

    assert_eq!(
        game.players[PlayerId::One.index()]
            .hand
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::GIANT_GROWTH],
        "the card is discarded as a cost and one is drawn",
    );
    assert_eq!(hand_before, 1);
    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::UNEARTH),
    );
}

/// "If a card in a player's graveyard has {X} in its mana cost, X is
/// considered to be 0." A Walking Ballista costs {X}{X} and so sits in the
/// graveyard at mana value nought, well inside the bound -- and X is still
/// zero when it arrives, so it enters as a 0/0 and the game rules collect
/// it again immediately. A Mirrorworks watching for artifacts is what shows
/// it got there at all.
#[test]
fn an_x_creature_is_worth_nothing_in_the_graveyard() {
    let (mut game, unearth) = staged(&[cards::WALKING_BALLISTA]);
    game.put_onto_battlefield(PlayerId::One, cards::MIRRORWORKS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;
    let ballista = graveyard_card(&game, cards::WALKING_BALLISTA);

    let cast = cast_at(&game, unearth, ballista).expect("mana value zero is three or less");
    game.apply(PlayerId::One, cast).expect("it is cast");

    let mut watched = false;
    for _ in 0..8 {
        if let Some(pending) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            watched |= pending
                .prompt
                .starts_with("Whenever another nontoken artifact");
            let options = pending
                .options
                .iter()
                .find(|option| option.label == "Decline")
                .map(|option| vec![option.id])
                .unwrap_or_default();
            game.apply(
                pending.player,
                Action::ChooseDecision {
                    decision: pending.id,
                    options,
                },
            )
            .expect("declining is allowed");
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

    assert!(watched, "the Ballista entered the battlefield");
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::WALKING_BALLISTA),
        "and left it again: X is zero here too, so it is a 0/0",
    );
    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::WALKING_BALLISTA),
        "back where it came from, having been on the battlefield in between",
    );
}

/// "From *your* graveyard": the yard across the table is no part of it,
/// however small what is lying in it. With their Bears there and yours not,
/// the spell has nothing to name and only the cycling is on offer.
#[test]
fn it_cannot_reach_their_graveyard() {
    let (mut game, unearth) = staged(&[]);
    game.players[PlayerId::Two.index()].graveyard.clear();
    game.players[PlayerId::Two.index()].graveyard.push(card(
        85_400,
        cards::GRIZZLY_BEARS,
        PlayerId::Two,
    ));
    let theirs = game.players[PlayerId::Two.index()].graveyard[0].id;

    assert!(
        cast_at(&game, unearth, theirs).is_none(),
        "their creature is not a card the spell may name",
    );
    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == unearth)),
        "and with nothing of yours down there it cannot be cast at all",
    );
}

/// Cycling is an activated ability and the spell is a sorcery, so on their
/// turn the card is still worth something: it cannot raise the Bears until
/// your own main phase, but it can always be cashed in for a card.
#[test]
fn on_their_turn_only_the_cycling_is_left() {
    let (mut game, unearth) = staged(&[cards::GRIZZLY_BEARS]);
    let bears = graveyard_card(&game, cards::GRIZZLY_BEARS);
    assert!(
        cast_at(&game, unearth, bears).is_some(),
        "on your own main phase it raises them",
    );

    game.turn += 1;
    game.active_player = PlayerId::Two;
    game.turns_started[PlayerId::Two.index()] += 1;
    game.step = Step::End;
    game.priority = PlayerId::One;

    assert!(
        cast_at(&game, unearth, bears).is_none(),
        "a sorcery waits for a turn of your own",
    );
    let cycle = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == unearth),
        )
        .expect("cycling waits for nothing");
    game.apply(PlayerId::One, cycle).expect("it activates");
    resolve(&mut game);

    assert!(
        game.players[PlayerId::One.index()]
            .hand
            .iter()
            .any(|card| card.definition == cards::GIANT_GROWTH),
        "so the card drew you one on their end step instead",
    );
}
