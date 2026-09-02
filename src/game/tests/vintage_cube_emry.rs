//! Emry, Lurker of the Loch: cheap on an artifact board, and the mill she
//! arrives with is where she finds what to recast.

use super::*;

/// Player One holding Emry, with `board` under them and `graveyard` in
/// their graveyard.
fn staged(board: &[CardDefinitionId], graveyard: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    game.players[0].graveyard.clear();
    for index in 0..8 {
        game.players[0]
            .library
            .push(card(117_000 + index, cards::ISLAND, PlayerId::One));
    }
    for (index, definition) in graveyard.iter().enumerate() {
        let id = 117_100 + u32::try_from(index).expect("a short graveyard");
        game.players[0]
            .graveyard
            .push(card(id, *definition, PlayerId::One));
    }
    for definition in board {
        game.put_onto_battlefield(PlayerId::One, *definition)
            .expect("cataloged");
    }
    drain_pending(&mut game);
    let card = game
        .build_zone(PlayerId::One, &[cards::EMRY_LURKER_OF_THE_LOCH])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let held = card.id;
    game.players[0].hand.push(card);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, held)
}

/// The mana costs of every way Emry can be cast right now.
fn casts(game: &Game, held: GameObjectId) -> Vec<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(|action| matches!(action, Action::CastSpell { card, .. } if *card == held))
        .collect()
}

/// Affinity: two artifacts take two off the three.
#[test]
fn affinity_pays_for_her() {
    let (mut game, held) = staged(&[cards::HOWLING_MINE, cards::MANIFOLD_KEY], &[]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);

    assert!(
        !casts(&game, held).is_empty(),
        "one blue mana is the whole of what is left",
    );
}

/// Without the artifacts the discount is not there.
#[test]
fn an_empty_board_pays_full_price() {
    let (mut game, held) = staged(&[], &[]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);

    assert!(casts(&game, held).is_empty(), "one mana is not three");

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);
    assert!(!casts(&game, held).is_empty(), "three of them is");
}

/// She arrives and mills four.
#[test]
fn she_mills_four_on_arrival() {
    let (mut game, held) = staged(&[cards::HOWLING_MINE, cards::MANIFOLD_KEY], &[]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    let library = game.players[0].library.len();

    let cast = casts(&game, held)
        .into_iter()
        .next()
        .expect("she is castable");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);

    assert_eq!(game.players[0].library.len(), library - 4);
    assert_eq!(game.players[0].graveyard.len(), 4, "four cards milled");
}

/// Tapping her makes an artifact card in the graveyard castable, and it is
/// cast from the graveyard for its own cost.
#[test]
fn her_tap_recasts_an_artifact() {
    let (mut game, held) = staged(
        &[cards::HOWLING_MINE, cards::MANIFOLD_KEY],
        &[cards::BLACK_LOTUS],
    );
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    let cast = casts(&game, held)
        .into_iter()
        .next()
        .expect("she is castable");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);
    let emry = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::EMRY_LURKER_OF_THE_LOCH)
        .expect("she is there")
        .card
        .id;
    // She may not tap the turn she arrives; the ability is what is being
    // tested rather than summoning sickness.
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    let lotus = game.players[0]
        .graveyard
        .iter()
        .find(|card| card.definition == cards::BLACK_LOTUS)
        .expect("the Lotus is in the graveyard")
        .id;

    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == lotus)),
        "a card in the graveyard is not castable on its own",
    );

    let tap = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } => {
                *source == emry
                    && targets
                        .iter()
                        .any(|selection| selection.targets().contains(&Target::Card(lotus)))
            }
            _ => false,
        })
        .expect("she can point at the Lotus");
    game.apply(PlayerId::One, tap).expect("it activates");
    drain_pending(&mut game);

    let recast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == lotus))
        .expect("and now it may be cast from the graveyard");
    game.apply(PlayerId::One, recast).expect("it is cast");
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::BLACK_LOTUS),
        "the Lotus is on the battlefield",
    );
}

/// "Artifact card in your graveyard": a creature card in it is not one.
#[test]
fn a_nonartifact_card_is_not_a_target() {
    let (mut game, held) = staged(
        &[cards::HOWLING_MINE, cards::MANIFOLD_KEY],
        &[cards::GRIZZLY_BEARS],
    );
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    let cast = casts(&game, held)
        .into_iter()
        .next()
        .expect("she is castable");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);
    let emry = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::EMRY_LURKER_OF_THE_LOCH)
        .expect("she is there")
        .card
        .id;
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    let bears = game.players[0]
        .graveyard
        .iter()
        .find(|card| card.definition == cards::GRIZZLY_BEARS)
        .expect("the bear is in the graveyard")
        .id;

    assert!(
        !game.legal_actions(PlayerId::One).iter().any(|action| {
            matches!(
                action,
                Action::ActivateAbility { source, targets, .. }
                    if *source == emry
                        && targets.iter().any(|selection| {
                            selection.targets().contains(&Target::Card(bears))
                        })
            )
        }),
        "a creature card is not an artifact card",
    );
}

/// "Reduces only the generic mana": five artifacts do not make her free, and
/// the blue pip is not something colourless mana can answer.
#[test]
fn affinity_never_eats_the_blue_pip() {
    let (mut game, held) = staged(
        &[
            cards::HOWLING_MINE,
            cards::MANIFOLD_KEY,
            cards::SOL_RING,
            cards::MOX_RUBY,
            cards::MOX_PEARL,
        ],
        &[],
    );

    assert!(casts(&game, held).is_empty(), "five artifacts are not five");

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 3);
    assert!(
        casts(&game, held).is_empty(),
        "and colourless mana does not pay a blue pip",
    );

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    assert!(
        !casts(&game, held).is_empty(),
        "the pip is all that is owed"
    );
}

/// "Casting the target card causes it to leave your graveyard and become a
/// new object": the Lotus that sacrifices itself for mana lands back in the
/// graveyard the same turn, and the permission does not follow it there.
#[test]
fn the_permission_does_not_survive_a_trip_back_to_the_graveyard() {
    let (mut game, _held) = staged(&[cards::HOWLING_MINE], &[cards::BLACK_LOTUS]);
    let emry = game
        .put_onto_battlefield(PlayerId::One, cards::EMRY_LURKER_OF_THE_LOCH)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    let lotus = game.players[0]
        .graveyard
        .iter()
        .find(|card| card.definition == cards::BLACK_LOTUS)
        .expect("it is buried")
        .id;

    let tap = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } => {
                *source == emry
                    && targets
                        .iter()
                        .any(|selection| selection.targets().contains(&Target::Card(lotus)))
            }
            _ => false,
        })
        .expect("she can point at the Lotus");
    game.apply(PlayerId::One, tap).expect("it activates");
    drain_pending(&mut game);
    let recast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == lotus))
        .expect("the permission is live");
    game.apply(PlayerId::One, recast).expect("it is cast");
    drain_pending(&mut game);

    let on_battlefield = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::BLACK_LOTUS)
        .expect("the Lotus resolved")
        .card
        .id;
    let sacrifice = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateManaAbility { source, .. } if *source == on_battlefield)
        })
        .expect("it taps itself for mana");
    game.apply(PlayerId::One, sacrifice).expect("it is spent");
    drain_pending(&mut game);

    let returned = game.players[0]
        .graveyard
        .iter()
        .find(|card| card.definition == cards::BLACK_LOTUS)
        .expect("it sacrificed itself back into the graveyard")
        .id;
    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == returned)),
        "what came back is a new object the permission never named",
    );
}

/// Puts Emry on the battlefield ready to tap, with `graveyard` behind her,
/// and returns her and the artifact card she will name.
fn ready_to_tap(graveyard: CardDefinitionId) -> (Game, GameObjectId, CardInstanceId) {
    let (mut game, _held) = staged(&[], &[graveyard]);
    let emry = game
        .put_onto_battlefield(PlayerId::One, cards::EMRY_LURKER_OF_THE_LOCH)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    let card = game.players[0]
        .graveyard
        .iter()
        .find(|card| card.definition == graveyard)
        .expect("it is in the graveyard")
        .id;
    game.priority = PlayerId::One;
    (game, emry, card)
}

/// Points her at `card` and lets the ability resolve.
fn name_it(game: &mut Game, emry: GameObjectId, card: CardInstanceId) {
    let tap = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } => {
                *source == emry
                    && targets
                        .iter()
                        .any(|selection| selection.targets().contains(&Target::Card(card)))
            }
            _ => false,
        })
        .expect("she can point at it");
    game.apply(PlayerId::One, tap).expect("it activates");
    drain_pending(game);
}

/// "You must follow the normal timing permissions and restrictions for the
/// target artifact card." The permission is a cast, not a time to cast: an
/// ordinary artifact waits for your main phase.
#[test]
fn the_permission_is_no_licence_to_cast_it_whenever() {
    let (mut game, emry, lotus) = ready_to_tap(cards::HOWLING_MINE);
    name_it(&mut game, emry, lotus);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 3);
    let castable = |game: &Game| {
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == lotus))
    };
    assert!(castable(&game), "your own main phase casts it");

    game.step = Step::DeclareBlockers;
    assert!(!castable(&game), "combat is not a window for an artifact");

    game.step = Step::PrecombatMain;
    game.active_player = PlayerId::Two;
    assert!(!castable(&game), "and neither is their turn");
}

/// "The mana value of the spell remains unchanged, no matter what the total
/// cost to cast it was." Affinity takes two off what Emry costs and nothing
/// off what she is worth.
#[test]
fn affinity_leaves_her_mana_value_alone() {
    let (mut game, held) = staged(&[cards::HOWLING_MINE, cards::MANIFOLD_KEY], &[]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);

    let cast = casts(&game, held)
        .into_iter()
        .next()
        .expect("one blue is the whole price with two artifacts out");
    game.apply(PlayerId::One, cast).expect("it is cast");
    let spell = game
        .stack
        .iter()
        .next()
        .map(|object| object.id)
        .expect("she is on the stack");

    assert_eq!(
        game.current_or_last_known_mana_value(spell),
        Some(3),
        "{{2}}{{U}} is three however little of it was paid",
    );
}

/// "For each artifact *you control*." Their board is not your affinity,
/// however many rocks are sitting on it.
#[test]
fn their_artifacts_do_not_pay_for_her() {
    let (mut game, held) = staged(&[], &[]);
    for definition in [cards::HOWLING_MINE, cards::MANIFOLD_KEY] {
        game.put_onto_battlefield(PlayerId::Two, definition)
            .expect("cataloged");
    }
    drain_pending(&mut game);
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);

    assert!(
        casts(&game, held).is_empty(),
        "two of their artifacts take nothing off",
    );

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);
    assert!(
        !casts(&game, held).is_empty(),
        "and she still costs the full three",
    );
}

/// An artifact is an artifact however it got there: two Treasures count for
/// affinity exactly as two artifact cards do.
#[test]
fn artifact_tokens_pay_for_her_too() {
    let (mut game, held) = staged(&[], &[]);
    for _ in 0..2 {
        game.create_token(PlayerId::One, tokens::treasure());
    }
    drain_pending(&mut game);
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);

    assert!(
        !casts(&game, held).is_empty(),
        "two Treasures are two artifacts",
    );
}

/// Her ability costs a tap, so the turn she lands she mills and does nothing
/// else: the Lotus she turned over is hers to name only from her next turn
/// on.
#[test]
fn the_turn_she_arrives_she_only_mills() {
    let (mut game, held) = staged(&[cards::HOWLING_MINE, cards::MANIFOLD_KEY], &[]);
    // An artifact on top, so what she mills is something she could name.
    game.players[0]
        .library
        .push(card(117_900, cards::BLACK_LOTUS, PlayerId::One));
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    let cast = casts(&game, held)
        .into_iter()
        .next()
        .expect("she is castable");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);

    let emry = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::EMRY_LURKER_OF_THE_LOCH)
        .expect("she arrived")
        .card
        .id;
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::BLACK_LOTUS),
        "the Lotus she milled is sitting there",
    );
    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(
                action,
                Action::ActivateAbility { source, .. } if *source == emry
            )),
        "and her tap is a creature's tap, waiting a turn",
    );

    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == emry)
    {
        permanent.entered_controller_turn = 0;
    }

    assert!(
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(
                action,
                Action::ActivateAbility { source, .. } if *source == emry
            )),
        "which the next turn gives her",
    );
}

/// "You may cast that card *this turn*." A card named and left in the
/// graveyard is a card the permission has finished with: the turn ends and
/// it is an ordinary graveyard card again, waiting for her to name it a
/// second time.
#[test]
fn the_permission_lapses_with_the_turn_it_was_given_in() {
    let (mut game, emry, mine) = ready_to_tap(cards::HOWLING_MINE);
    name_it(&mut game, emry, mine);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 3);
    let castable = |game: &Game| {
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == mine))
    };
    assert!(castable(&game), "named, and castable while the turn lasts");

    // Walking the steps round to a turn of yours again rather than jumping
    // there: the permission is keyed to the turn it was given in.
    let started = game.turns_started[PlayerId::One.index()];
    for _ in 0..160 {
        if game.turns_started[PlayerId::One.index()] > started {
            break;
        }
        game.advance_step();
        drain_pending(&mut game);
    }
    assert!(
        game.turns_started[PlayerId::One.index()] > started,
        "a turn of theirs and back round to one of yours",
    );
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 3);

    assert!(
        !castable(&game),
        "and on a later main phase of yours it is not castable at all",
    );
    assert!(
        game.players[0].graveyard.iter().any(|card| card.id == mine),
        "the card is where it always was",
    );

    // She may simply name it again, which is what the tap is for.
    for permanent in &mut game.battlefield {
        permanent.tapped = false;
    }
    name_it(&mut game, emry, mine);
    assert!(castable(&game), "a second naming buys the same turn again");
}
