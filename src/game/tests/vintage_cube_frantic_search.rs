//! Frantic Search: two cards deep, two cards back, and the three mana
//! returned -- which is what makes it free, and what makes the lands it
//! untaps worth reading carefully.

use super::*;

/// Player One holding the Search with three mana up, `mine` tapped lands of
/// their own and `theirs` tapped across the table, and a library to draw
/// from.
fn staged(mine: usize, theirs: usize) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for index in 0..4 {
        game.players[0]
            .library
            .push(card(125_000 + index, cards::COUNTERSPELL, PlayerId::One));
    }
    for (player, count) in [(PlayerId::One, mine), (PlayerId::Two, theirs)] {
        for index in 0..count {
            let mut land = creature(
                125_100
                    + u32::from(player == PlayerId::Two) * 50
                    + u32::try_from(index).expect("a few lands"),
                cards::ISLAND,
                player,
            );
            land.tapped = true;
            game.battlefield.push(land);
        }
    }
    let search = card(125_200, cards::FRANTIC_SEARCH, PlayerId::One);
    let search_id = search.id;
    game.players[0].hand.push(search);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, search_id)
}

/// Casts it and stops at the decision it asks.
fn cast(game: &mut Game, search: GameObjectId) -> DecisionObservation {
    game.apply(
        PlayerId::One,
        cast_action(search, Vec::new(), Vec::new(), 0),
    )
    .expect("three mana casts it");
    pass_until_decision(game);
    game.observe(PlayerId::One)
        .decision
        .expect("something is being asked")
}

fn untapped_of(game: &Game, player: PlayerId) -> usize {
    game.battlefield
        .iter()
        .filter(|permanent| permanent.controller == player && !permanent.tapped)
        .count()
}

/// "They don't have to be lands that you control": their tapped Island is
/// as good a choice as yours, whatever you would want that for.
#[test]
fn their_lands_are_choices_too() {
    let (mut game, search) = staged(1, 2);

    let decision = cast(&mut game, search);
    assert_eq!(
        decision.options.len(),
        3,
        "every tapped land on the battlefield is on offer",
    );
    let theirs = decision
        .options
        .iter()
        .filter(|option| {
            option.card.is_some_and(|(id, _)| {
                game.battlefield.iter().any(|permanent| {
                    permanent.card.id == id && permanent.controller == PlayerId::Two
                })
            })
        })
        .map(|option| option.id)
        .collect::<Vec<_>>();
    assert_eq!(theirs.len(), 2, "two of them are theirs");

    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: theirs,
        },
    )
    .expect("nothing about the choice asks who controls them");
    drain_pending(&mut game);

    assert_eq!(untapped_of(&game, PlayerId::Two), 2, "theirs came back up");
    assert_eq!(
        untapped_of(&game, PlayerId::One),
        0,
        "and yours stayed where it was",
    );
}

/// "Up to three": fewer is an answer, and none at all is one too.
#[test]
fn untapping_nothing_is_an_answer() {
    let (mut game, search) = staged(3, 0);
    let hand = game.players[0].hand.len();

    let decision = cast(&mut game, search);
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: Vec::new(),
        },
    )
    .expect("up to three is a maximum");
    drain_pending(&mut game);

    assert_eq!(untapped_of(&game, PlayerId::One), 0, "nothing came back up");
    assert_eq!(
        game.players[0].hand.len(),
        hand - 1,
        "and the draws and discards still cancelled out",
    );
}

/// "Nothing can happen between the two, and no player may choose to take
/// actions": the discard is part of the resolution.
#[test]
fn nobody_acts_between_the_draws_and_the_discards() {
    let (mut game, search) = staged(1, 0);
    game.players[0]
        .hand
        .push(card(125_300, cards::MOUNTAIN, PlayerId::One));
    game.players[0]
        .hand
        .push(card(125_301, cards::FOREST, PlayerId::One));

    let decision = cast(&mut game, search);
    assert!(
        decision.prompt.to_lowercase().contains("discard"),
        "the discard is what it stops on: {}",
        decision.prompt,
    );
    assert_eq!(
        game.legal_actions(PlayerId::Two),
        vec![Action::Concede],
        "and the other player has no window while it waits",
    );
}

/// The line the card is played for, counted out: two cards deep, two cards
/// back, and the ones you keep are the ones you drew if that is what you
/// choose.
#[test]
fn it_draws_two_and_discards_two() {
    let (mut game, search) = staged(3, 0);
    game.players[0]
        .hand
        .push(card(125_400, cards::MOUNTAIN, PlayerId::One));
    game.players[0]
        .hand
        .push(card(125_401, cards::FOREST, PlayerId::One));
    let library = game.players[0].library.len();

    let discard = cast(&mut game, search);
    assert!(
        discard.prompt.to_lowercase().contains("discard"),
        "the discard is asked first: {}",
        discard.prompt,
    );
    let land_options = discard
        .options
        .iter()
        .filter(|option| {
            option.card.is_some_and(|(_, characteristics)| {
                matches!(
                    characteristics.card_definition(),
                    Some(definition) if definition == cards::MOUNTAIN || definition == cards::FOREST
                )
            })
        })
        .map(|option| option.id)
        .collect::<Vec<_>>();
    assert_eq!(
        land_options.len(),
        2,
        "the two lands are what you would pitch"
    );
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: discard.id,
            options: land_options,
        },
    )
    .expect("two of the four in hand");
    drain_pending(&mut game);

    assert_eq!(
        game.players[0].library.len(),
        library - 2,
        "two cards off the top",
    );
    assert_eq!(
        game.players[0]
            .hand
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::COUNTERSPELL, cards::COUNTERSPELL],
        "and the two drawn are what is left",
    );
    assert_eq!(
        game.players[0]
            .graveyard
            .iter()
            .filter(|card| card.definition != cards::FRANTIC_SEARCH)
            .count(),
        2,
        "with the two pitched behind them",
    );
}

/// "You choose which lands to untap as the spell resolves. They aren't
/// targeted." Nothing is named as it is cast: the lands are picked out of a
/// decision once it is resolving.
#[test]
fn the_lands_are_chosen_on_resolution_and_never_targeted() {
    let (game, search) = staged(3, 0);

    let casts = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter(|action| matches!(action, Action::CastSpell { card, .. } if *card == search))
        .collect::<Vec<_>>();
    assert!(!casts.is_empty(), "three mana casts it");
    for action in &casts {
        let Action::CastSpell { choices, .. } = action else {
            unreachable!("filtered to casts")
        };
        assert_eq!(
            choices.iter_targets().count(),
            0,
            "a cast of it names nothing at all",
        );
    }
}

/// The whole reason it is played: pay with three lands, untap those same
/// three, and the spell has cost nothing but the cards it churned.
#[test]
fn the_lands_that_paid_for_it_come_back_up() {
    let (mut game, search) = staged(3, 0);
    // The fixture hands the mana over; this line wants the lands to be what
    // pays, so the pool starts empty and the Islands are untapped.
    game.players[0].mana_pool = ManaPool::default();
    let islands = game
        .battlefield
        .iter_mut()
        .filter(|permanent| permanent.controller == PlayerId::One)
        .map(|permanent| {
            permanent.tapped = false;
            permanent.card.id
        })
        .collect::<Vec<_>>();
    assert_eq!(islands.len(), 3, "three Islands and nothing else");

    for island in &islands {
        game.apply(
            PlayerId::One,
            Action::ActivateManaAbility {
                source: *island,
                ability: mana_ability_for(&game, *island, ManaColor::Blue),
                color: ManaColor::Blue,
                counters_removed: None,
                cost_object: None,
                combination: None,
                triggered_mana: None,
            },
        )
        .expect("each taps for blue");
    }
    assert_eq!(untapped_of(&game, PlayerId::One), 0, "all three are tapped");

    // The hand is the Search alone, so the two cards drawn are the only two
    // that can be discarded and nobody is asked which. The lands are the
    // whole question.
    let untap = cast(&mut game, search);
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: untap.id,
            options: untap.options.iter().map(|option| option.id).collect(),
        },
    )
    .expect("all three of them");
    drain_pending(&mut game);

    assert_eq!(
        untapped_of(&game, PlayerId::One),
        3,
        "the mana it cost is standing back up",
    );
    assert_eq!(
        game.players[0].mana_pool.total(),
        0,
        "and the three blue it was paid with is spent, not refunded",
    );
}

/// "Untap up to three lands" says nothing about tapped ones: an untapped
/// land is a legal choice and untapping it changes nothing, which is what
/// the count being of lands rather than of tapped lands means.
#[test]
fn an_untapped_land_is_a_legal_choice() {
    let (mut game, search) = staged(1, 0);
    let ready = game
        .put_onto_battlefield(PlayerId::One, cards::MOUNTAIN)
        .expect("cataloged");
    drain_pending(&mut game);

    // Draw and discard first; the land choice is the decision after those.
    let mut decision = cast(&mut game, search);
    for _ in 0..4 {
        if decision
            .options
            .iter()
            .any(|option| option.card.is_some_and(|(id, _)| id == ready))
        {
            break;
        }
        game.apply(
            PlayerId::One,
            Action::ChooseDecision {
                decision: decision.id,
                options: decision
                    .options
                    .iter()
                    .map(|option| option.id)
                    .take(decision.minimum)
                    .collect(),
            },
        )
        .expect("the offered answer is legal");
        pass_until_decision(&mut game);
        decision = game
            .observe(PlayerId::One)
            .decision
            .expect("the lands are asked about");
    }

    let option = decision
        .options
        .iter()
        .find(|option| option.card.is_some_and(|(id, _)| id == ready))
        .expect("the untapped Mountain is among the choices")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![option],
        },
    )
    .expect("choosing it is legal");
    drain_pending(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == ready)
            .expect("it is there")
            .tapped,
        "it was untapped before and is untapped after",
    );
    assert_eq!(
        untapped_of(&game, PlayerId::One),
        1,
        "and the tapped Island was not the one chosen",
    );
}

/// Two cards off a library with one in it: the draw takes what is there, and
/// the loss is the state-based check afterwards rather than anything the
/// spell does.
#[test]
fn drawing_past_the_end_of_the_library_loses_the_game() {
    let (mut game, search) = staged(3, 0);
    game.players[0].library.truncate(1);

    game.apply(
        PlayerId::One,
        cast_action(search, Vec::new(), Vec::new(), 0),
    )
    .expect("three mana casts it");
    pass_until_decision(&mut game);
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert!(
        game.players[0].library.is_empty(),
        "the one card it could draw was drawn",
    );
    assert_eq!(
        game.result,
        Some(GameResult::Winner {
            winner: PlayerId::Two,
            reason: WinReason::OpponentTriedToDrawFromEmptyLibrary,
        }),
        "and drawing from an empty library is a loss",
    );
}

/// "Up to three" is a ceiling as well as a licence. Six tapped lands are all
/// offered and any three of them may come back, but the fourth is not on the
/// table: a board wider than the clause does not untap itself.
#[test]
fn three_is_as_many_as_it_untaps() {
    let (mut game, search) = staged(4, 2);

    let decision = cast(&mut game, search);
    assert_eq!(decision.options.len(), 6, "every land is a candidate");
    assert_eq!(decision.maximum, 3, "and three of them is the most");
    assert_eq!(decision.minimum, 0, "with none of them allowed too");

    let four = decision
        .options
        .iter()
        .take(4)
        .map(|option| option.id)
        .collect::<Vec<_>>();
    assert!(
        game.apply(
            PlayerId::One,
            Action::ChooseDecision {
                decision: decision.id,
                options: four,
            },
        )
        .is_err(),
        "a fourth land is not something the clause offers",
    );

    let three = decision
        .options
        .iter()
        .take(3)
        .map(|option| option.id)
        .collect::<Vec<_>>();
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: three,
        },
    )
    .expect("three is the number");
    drain_pending(&mut game);

    assert_eq!(
        untapped_of(&game, PlayerId::One) + untapped_of(&game, PlayerId::Two),
        3,
        "three came up and the other three stayed down",
    );
}

/// "Untap up to three *lands*": a tapped creature is not one of them, and
/// neither is a tapped artifact. The Monolith beside the Island is exactly
/// the permanent a player would most like this to reach, and the clause does
/// not reach it.
#[test]
fn only_lands_are_on_the_menu() {
    let (mut game, search) = staged(1, 0);
    let bears = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    let monolith = game
        .put_onto_battlefield(PlayerId::One, cards::BASALT_MONOLITH)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.tapped = true;
    }
    game.priority = PlayerId::One;

    let decision = cast(&mut game, search);
    let offered = decision
        .options
        .iter()
        .filter_map(|option| option.card.map(|(object, _)| object))
        .collect::<Vec<_>>();

    assert!(!offered.contains(&bears), "a tapped creature is not a land",);
    assert!(
        !offered.contains(&monolith),
        "and neither is the artifact that would most like to be one",
    );
    assert!(
        offered.iter().all(|object| game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == *object)
            .is_some_and(|permanent| permanent.card.definition == cards::ISLAND)),
        "the Island is the whole of the offer: {offered:?}",
    );
}
