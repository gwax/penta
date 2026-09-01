//! Skyclave Apparition: an answer that only comes undone by killing it, and
//! then only into an Illusion.

use super::*;

/// The Apparition in hand with three mana, and whatever the other player has
/// out already on the battlefield.
fn staged(theirs: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    for definition in theirs {
        game.put_onto_battlefield(PlayerId::Two, *definition)
            .expect("cataloged");
    }
    drain_pending(&mut game);
    let apparition = game
        .build_zone(PlayerId::One, &[cards::SKYCLAVE_APPARITION])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let apparition_id = apparition.id;
    game.players[0].hand.push(apparition);
    game.turns_started = [2, 1];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 3);
    (game, apparition_id)
}

fn settle(game: &mut Game) {
    for _ in 0..24 {
        if !game.pending_decisions.is_empty() {
            return;
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

/// Casts the Apparition, aiming its trigger at `victim` when one is given.
fn cast_apparition(game: &mut Game, card: GameObjectId, victim: Option<GameObjectId>) {
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card: cast, .. } if *cast == card))
        .expect("three mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    for _ in 0..12 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .filter(|option| match victim {
                    Some(victim) => option.card.is_some_and(|(card, _)| card == victim),
                    None => option.card.is_none(),
                })
                .map(|option| option.id)
                .take(1)
                .collect::<Vec<_>>();
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the choice is legal");
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

/// The permanents the enters trigger may name, once it is on the stack.
fn exile_targets(game: &Game) -> Vec<GameObjectId> {
    game.pending_decisions
        .first()
        .into_iter()
        .flat_map(|pending| pending.observation.options.iter())
        .filter_map(|option| option.card.map(|(card, _)| card))
        .collect()
}

fn apparition_on_battlefield(game: &Game) -> GameObjectId {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::SKYCLAVE_APPARITION)
        .expect("the Apparition resolved")
        .card
        .id
}

fn illusions(game: &Game) -> Vec<&Permanent> {
    game.battlefield
        .iter()
        .filter(|permanent| permanent.card.definition == ObjectKind::Token)
        .collect()
}

/// It takes an opponent's cheap nonland permanent, and the card stays gone.
#[test]
fn it_exiles_a_cheap_permanent_they_control() {
    let (mut game, apparition) = staged(&[cards::GRIZZLY_BEARS]);
    let bears = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::GRIZZLY_BEARS)
        .expect("they have Bears")
        .card
        .id;

    cast_apparition(&mut game, apparition, Some(bears));

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == bears),
        "the Bears are gone",
    );
    assert!(
        game.players[1]
            .exile
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS),
        "and they are in exile, not a graveyard",
    );
    assert!(illusions(&game).is_empty(), "with nothing given back yet");
}

/// A land, a token, something too expensive, and its own controller's
/// permanents are all safe.
#[test]
fn it_refuses_lands_tokens_and_expensive_permanents() {
    let (mut game, apparition) = staged(&[
        cards::MISHRA_S_FACTORY,
        cards::SHIVAN_DRAGON,
        cards::GRIZZLY_BEARS,
    ]);
    let mine = creature(99_100, cards::SAVANNAH_LIONS, PlayerId::One);
    let mine_id = mine.card.id;
    game.battlefield.push(mine);
    // A token they control, which is theirs and cheap and still not a legal
    // target.
    game.create_token(
        PlayerId::Two,
        crate::card::TokenCharacteristics::creature(&["Bear"], &[ManaColor::Green], 2, 2),
    );
    drain_pending(&mut game);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == apparition))
        .expect("three mana casts it");
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

    let offered = exile_targets(&game);
    let bears = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::GRIZZLY_BEARS)
        .expect("they have Bears")
        .card
        .id;
    assert_eq!(offered, vec![bears], "only the cheap nonland nontoken");
    assert!(!offered.contains(&mine_id), "and never your own");
}

/// Killing it hands the card's owner an Illusion the size of what it took,
/// and the card itself stays in exile.
#[test]
fn leaving_pays_the_owner_an_illusion_of_that_size() {
    let (mut game, apparition) = staged(&[cards::SHIVAN_DRAGON, cards::ICY_MANIPULATOR]);
    // The Manipulator costs four: the most expensive thing it can take.
    let manipulator = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::ICY_MANIPULATOR)
        .expect("they have it")
        .card
        .id;
    cast_apparition(&mut game, apparition, Some(manipulator));

    let body = apparition_on_battlefield(&game);
    game.destroy_permanent(body);
    settle(&mut game);

    let made = illusions(&game);
    assert_eq!(made.len(), 1, "one token");
    assert_eq!(made[0].controller, PlayerId::Two, "for the card's owner");
    assert_eq!(game.power(made[0]), Some(4), "the size of what it took");
    assert_eq!(game.toughness(made[0]), Some(4));
    assert!(
        game.players[1]
            .exile
            .iter()
            .any(|card| card.definition == cards::ICY_MANIPULATOR),
        "and the card itself stays exiled",
    );
}

/// "Up to one": an Apparition that took nothing owes nobody a token.
#[test]
fn taking_nothing_pays_nothing() {
    let (mut game, apparition) = staged(&[cards::GRIZZLY_BEARS]);

    cast_apparition(&mut game, apparition, None);
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::GRIZZLY_BEARS),
        "nothing was exiled",
    );

    let body = apparition_on_battlefield(&game);
    game.destroy_permanent(body);
    settle(&mut game);

    assert!(illusions(&game).is_empty(), "and nothing is given back");
}

/// Two rulings that meet: "if it leaves the battlefield before its first
/// ability resolves, the ability still exiles the target permanent", and
/// "if there's no exiled card when it leaves, no player creates a token."
/// Answering it in response takes the Illusion away and not the exile.
#[test]
fn answering_it_in_response_exiles_without_paying() {
    let (mut game, apparition) = staged(&[cards::ICY_MANIPULATOR]);
    let manipulator = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::ICY_MANIPULATOR)
        .expect("they have it")
        .card
        .id;

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == apparition))
        .expect("three mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");

    // Resolve the body and answer the target choice, then kill it with the
    // exile still on the stack.
    for _ in 0..12 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .filter(|option| option.card.is_some_and(|(card, _)| card == manipulator))
                .map(|option| option.id)
                .take(1)
                .collect::<Vec<_>>();
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the choice is legal");
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    let body = apparition_on_battlefield(&game);
    game.destroy_permanent(body);
    settle(&mut game);

    assert!(
        game.players[1]
            .exile
            .iter()
            .any(|card| card.definition == cards::ICY_MANIPULATOR),
        "the exile happened even though its source had gone",
    );
    assert!(
        illusions(&game).is_empty(),
        "and there was nothing exiled yet when it left, so nobody was paid",
    );
}

/// "If a creature on the battlefield has {X} in its mana cost, X is
/// considered to be 0." A Walking Ballista is a nought-drop while it stands
/// there, so what it owes back is a 0/0 -- a token that arrives and is
/// buried by state-based actions before anyone can use it.
#[test]
fn an_x_cost_creature_pays_back_nothing_that_lives() {
    let (mut game, apparition) = staged(&[cards::WALKING_BALLISTA]);
    let ballista = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::WALKING_BALLISTA)
        .expect("they have it")
        .card
        .id;
    // Put onto the battlefield rather than cast, it arrives with no counters
    // at all, and a 0/0 does not survive the first state-based check. Two
    // counters give it a body without giving it a mana cost.
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == ballista)
        .expect("it is there")
        .add_counters(CounterKind::PlusOnePlusOne, 2);
    game.check_state_based_actions();

    cast_apparition(&mut game, apparition, Some(ballista));
    assert!(
        game.players[1]
            .exile
            .iter()
            .any(|card| card.definition == cards::WALKING_BALLISTA),
        "an X spell on the battlefield is a nought-drop and well within four",
    );

    let body = apparition_on_battlefield(&game);
    game.destroy_permanent(body);
    settle(&mut game);

    assert!(
        illusions(&game).is_empty(),
        "a 0/0 Illusion is one state-based check from the graveyard",
    );
    assert!(
        game.players[1]
            .exile
            .iter()
            .any(|card| card.definition == cards::WALKING_BALLISTA),
        "and the card it took stays where it was put",
    );
}

/// "When this creature leaves the battlefield" is leaves, not dies: bouncing
/// the Apparition to its owner's hand pays the Illusion just as killing it
/// does, and the card it took stays exiled either way.
#[test]
fn bouncing_it_pays_the_illusion_too() {
    let (mut game, apparition) = staged(&[cards::ICY_MANIPULATOR]);
    let manipulator = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::ICY_MANIPULATOR)
        .expect("they have it")
        .card
        .id;
    cast_apparition(&mut game, apparition, Some(manipulator));

    let body = apparition_on_battlefield(&game);
    game.return_permanent_to_hand(body);
    settle(&mut game);

    let made = illusions(&game);
    assert_eq!(made.len(), 1, "the leave trigger fired all the same");
    assert_eq!(made[0].controller, PlayerId::Two, "for the card's owner");
    assert_eq!(
        (game.power(made[0]), game.toughness(made[0])),
        (Some(4), Some(4)),
        "the size of what it took",
    );
    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::SKYCLAVE_APPARITION),
        "and the Apparition is back in hand rather than in a graveyard",
    );
    assert!(
        game.players[1]
            .exile
            .iter()
            .any(|card| card.definition == cards::ICY_MANIPULATOR),
        "while what it took is still gone",
    );
}

/// Exiling the Apparition itself is another way of leaving, and the answer
/// is the same: the Illusion is paid and the card it took does not come
/// back with it.
#[test]
fn exiling_it_pays_the_illusion_too() {
    let (mut game, apparition) = staged(&[cards::GRIZZLY_BEARS]);
    let bears = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::GRIZZLY_BEARS)
        .expect("they have it")
        .card
        .id;
    cast_apparition(&mut game, apparition, Some(bears));

    let body = apparition_on_battlefield(&game);
    game.exile_permanent(body);
    settle(&mut game);

    let made = illusions(&game);
    assert_eq!(made.len(), 1, "the leave trigger fired");
    assert_eq!(
        (game.power(made[0]), game.toughness(made[0])),
        (Some(2), Some(2)),
        "a two-drop is a 2/2 back",
    );
    assert!(
        game.players[1]
            .exile
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS),
        "and the bear stays where it was put",
    );
}
