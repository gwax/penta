//! Wight of the Reliquary: a body that grows with the graveyard it is
//! filling, and turns every spare creature into the land the deck needs.

use super::*;

/// The Wight on the battlefield since last turn, with `others` beside her,
/// `graveyard` behind her, and `library` to search.
fn staged(
    others: &[CardDefinitionId],
    graveyard: &[CardDefinitionId],
    library: &[CardDefinitionId],
) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    game.players[0].library.clear();
    for (index, definition) in graveyard.iter().enumerate() {
        game.players[0].graveyard.push(card(
            98_000 + u32::try_from(index).expect("a small graveyard"),
            *definition,
            PlayerId::One,
        ));
    }
    for (index, definition) in library.iter().enumerate() {
        game.players[0].library.push(card(
            98_500 + u32::try_from(index).expect("a small library"),
            *definition,
            PlayerId::One,
        ));
    }
    let wight = game
        .put_onto_battlefield(PlayerId::One, cards::WIGHT_OF_THE_RELIQUARY)
        .expect("cataloged");
    let mut ids = Vec::new();
    for definition in others {
        ids.push(
            game.put_onto_battlefield(PlayerId::One, *definition)
                .expect("cataloged"),
        );
    }
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [2, 1];
    drain_pending(&mut game);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, wight, ids)
}

fn settle(game: &mut Game) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .take(decision.minimum.max(1))
                .map(|option| option.id)
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
    game.check_state_based_actions();
}

fn permanent(game: &Game, id: GameObjectId) -> &Permanent {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("it is on the battlefield")
}

/// Every activation the Wight is offering, by what it would eat.
fn fetches(game: &Game, wight: GameObjectId) -> Vec<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == wight),
        )
        .collect()
}

/// She counts the creature cards in your own graveyard.
#[test]
fn she_grows_with_your_graveyard() {
    let (game, wight, _) = staged(
        &[],
        &[
            cards::GRIZZLY_BEARS,
            cards::SERRA_ANGEL,
            cards::LIGHTNING_BOLT,
        ],
        &[],
    );

    let wight = permanent(&game, wight);
    assert_eq!(game.power(wight), Some(4), "two creature cards, not three");
    assert_eq!(game.toughness(wight), Some(4));
}

/// Their graveyard is not yours.
#[test]
fn their_graveyard_does_not_count() {
    let (mut game, wight, _) = staged(&[], &[], &[]);
    game.players[1]
        .graveyard
        .push(card(98_900, cards::SERRA_ANGEL, PlayerId::Two));

    let wight = permanent(&game, wight);
    assert_eq!(game.power(wight), Some(2), "still a 2/2");
}

/// Eating a creature fetches a land onto the battlefield tapped, and the
/// creature it ate makes her bigger on the way past.
#[test]
fn sacrificing_a_creature_fetches_a_tapped_land() {
    let (mut game, wight, ids) = staged(&[cards::GRIZZLY_BEARS], &[], &[cards::FOREST]);

    let action = fetches(&game, wight)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility { cost_objects, .. } => cost_objects.contains(&ids[0]),
            _ => false,
        })
        .expect("the Bears can be eaten");
    game.apply(PlayerId::One, action).expect("it activates");
    settle(&mut game);

    let forest = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::FOREST)
        .expect("the Forest was found");
    assert!(forest.tapped, "and it arrives tapped");
    assert!(game.players[0].library.is_empty(), "out of the library");
    let wight = permanent(&game, wight);
    assert!(wight.tapped, "she tapped to do it");
    assert_eq!(
        game.power(wight),
        Some(3),
        "and the Bears she ate now count from the graveyard",
    );
}

/// "Another": she cannot eat herself, so a board with nothing else offers
/// no activation at all.
#[test]
fn she_cannot_eat_herself() {
    let (game, wight, _) = staged(&[], &[], &[cards::FOREST]);

    assert!(
        fetches(&game, wight).is_empty(),
        "there is no other creature to sacrifice",
    );
}

/// Vigilance: she attacks and is still untapped to pay for the fetch.
#[test]
fn vigilance_keeps_the_fetch_open() {
    let (mut game, wight, _) = staged(&[cards::GRIZZLY_BEARS], &[], &[cards::FOREST]);
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;

    game.apply(
        PlayerId::One,
        Action::DeclareAttacker {
            attacker: wight,
            defender: AttackDefender::Player(PlayerId::Two),
        },
    )
    .expect("she attacks");
    game.apply(PlayerId::One, Action::FinishDeclaringAttackers)
        .expect("the declaration finishes");
    drain_pending(&mut game);

    assert!(
        !permanent(&game, wight).tapped,
        "vigilance left her untapped",
    );
    assert!(
        !fetches(&game, wight).is_empty(),
        "so the fetch is still there to pay for",
    );
}

/// Her fetch costs a tap, and a tap is a creature's tap: a Wight that
/// arrived this turn offers nothing however many creatures stand beside her.
#[test]
fn a_fresh_wight_cannot_fetch() {
    let (mut game, wight, _) = staged(&[cards::GRIZZLY_BEARS], &[], &[cards::FOREST]);
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == wight)
    {
        permanent.entered_controller_turn = game.turns_started[0];
    }

    assert!(
        fetches(&game, wight).is_empty(),
        "she has to have been here since the turn began",
    );

    // One turn older and the same board pays for it.
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == wight)
    {
        permanent.entered_controller_turn = 0;
    }
    assert_eq!(
        fetches(&game, wight).len(),
        1,
        "the Bears beside her are food once she is settled",
    );
}

/// The tap and the creature are costs: a library with no land in it leaves
/// her tapped, the creature eaten, and nothing found -- and she is bigger
/// for it, which is the only thing the activation bought.
#[test]
fn a_landless_library_still_eats_the_creature() {
    let (mut game, wight, ids) = staged(&[cards::GRIZZLY_BEARS], &[], &[cards::LIGHTNING_BOLT]);

    let action = fetches(&game, wight)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility { cost_objects, .. } => cost_objects.contains(&ids[0]),
            _ => false,
        })
        .expect("nothing about the offer asks what the library holds");
    game.apply(PlayerId::One, action).expect("it activates");
    settle(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == ids[0]),
        "the Bears were eaten as a cost",
    );
    assert_eq!(
        game.players[0].library.len(),
        1,
        "and the Bolt stayed where it was",
    );
    let wight = permanent(&game, wight);
    assert!(wight.tapped, "she tapped for nothing");
    assert_eq!(
        game.power(wight),
        Some(3),
        "except the Bears in the graveyard, which is something",
    );
}

/// "A land card" and not a basic land card: the search offers everything in
/// the library that is a land, and none of what is not.
#[test]
fn the_search_finds_any_land_at_all() {
    let (mut game, wight, ids) = staged(
        &[cards::GRIZZLY_BEARS],
        &[],
        &[
            cards::LIGHTNING_BOLT,
            cards::FOREST,
            cards::GAEAS_CRADLE,
            cards::POLLUTED_DELTA,
        ],
    );

    let action = fetches(&game, wight)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility { cost_objects, .. } => cost_objects.contains(&ids[0]),
            _ => false,
        })
        .expect("the Bears can be eaten");
    game.apply(PlayerId::One, action).expect("it activates");
    pass_priority_pair(&mut game);

    let mut offered = game
        .observe(PlayerId::One)
        .decision
        .expect("the search asks which land to take")
        .options
        .iter()
        .filter_map(|option| {
            option
                .card
                .and_then(|(_, characteristics)| characteristics.card_definition())
        })
        .collect::<Vec<_>>();
    offered.sort_unstable();
    let mut expected = vec![cards::FOREST, cards::GAEAS_CRADLE, cards::POLLUTED_DELTA];
    expected.sort_unstable();

    assert_eq!(
        offered, expected,
        "a legend and a fetchland are land cards; the Bolt is not",
    );
}

/// "Each creature card in your graveyard" counts cards. A token she eats
/// ceases to exist rather than becoming one, so the meal pays for the land
/// and leaves her exactly as big as she was.
#[test]
fn a_token_she_eats_leaves_no_card_to_count() {
    let (mut game, wight, _) = staged(&[], &[], &[cards::FOREST]);
    let beast = tokens::creature(&["Beast"], &[ManaColor::Green], 3, 3);
    game.create_token(PlayerId::One, beast);
    drain_pending(&mut game);
    let token = game
        .battlefield
        .iter()
        .find(|permanent| is_token_with(permanent, beast))
        .expect("the Beast arrived")
        .card
        .id;
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.priority = PlayerId::One;
    assert_eq!(
        game.power(permanent(&game, wight)),
        Some(2),
        "a 2/2 to start"
    );

    let action = fetches(&game, wight)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility { cost_objects, .. } => cost_objects.contains(&token),
            _ => false,
        })
        .expect("a token is another creature she can eat");
    game.apply(PlayerId::One, action).expect("it activates");
    settle(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::FOREST),
        "the land was found",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .all(|card| card.definition != ObjectKind::Token),
        "and the token it cost ceased to exist",
    );
    assert_eq!(
        game.power(permanent(&game, wight)),
        Some(2),
        "so there is no creature card behind her to count",
    );
}

/// "Sacrifice another creature" is another creature *you control*: the one
/// across the table is no cost of yours.
#[test]
fn she_cannot_eat_their_creature() {
    let (mut game, wight, _) = staged(&[], &[], &[cards::FOREST]);
    game.put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    assert!(
        fetches(&game, wight).is_empty(),
        "their bear pays for nothing of hers",
    );

    // One of your own, and the activation appears.
    game.put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    assert_eq!(fetches(&game, wight).len(), 1, "and one of yours is a meal");
}

/// "For each creature card in your graveyard" is read live rather than
/// remembered: a creature card that leaves the graveyard takes its point
/// with it.
#[test]
fn a_creature_card_leaving_the_graveyard_shrinks_her() {
    let (mut game, wight, _) = staged(&[], &[cards::GRIZZLY_BEARS, cards::SERRA_ANGEL], &[]);
    assert_eq!(
        game.power(permanent(&game, wight)),
        Some(4),
        "two creature cards behind her",
    );

    let angel = game.players[0]
        .graveyard
        .iter()
        .position(|card| card.definition == cards::SERRA_ANGEL)
        .expect("it is there");
    let taken = game.players[0].graveyard.remove(angel);
    game.players[0].hand.push(taken);

    let wight = permanent(&game, wight);
    assert_eq!(
        (game.power(wight), game.toughness(wight)),
        (Some(3), Some(3)),
        "one card back in hand is one point off her",
    );
}
