//! Mightform Harmonizer: four mana for a 4/4 that makes every land drop a
//! pump spell, or three for one turn of it now and the card again later.

use super::*;

/// The Harmonizer in hand with the mana for either cost, `board` beside it,
/// and a land to play.
fn staged(board: &[CardDefinitionId]) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let mut ids = Vec::new();
    for definition in board {
        ids.push(
            game.put_onto_battlefield(PlayerId::One, *definition)
                .expect("cataloged"),
        );
    }
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    drain_pending(&mut game);
    let harmonizer = game
        .build_zone(PlayerId::One, &[cards::MIGHTFORM_HARMONIZER])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = harmonizer.id;
    game.players[0].hand.push(harmonizer);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 2);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.players[0].lands_played_this_turn = 0;
    (game, id, ids)
}

/// Answers whatever is waiting, naming `target` when one is wanted.
fn settle(game: &mut Game, target: Option<GameObjectId>) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = match target {
                Some(wanted) => decision
                    .options
                    .iter()
                    .filter(|option| option.card.is_some_and(|(object, _)| object == wanted))
                    .map(|option| option.id)
                    .take(1)
                    .collect(),
                None => decision
                    .options
                    .iter()
                    .take(decision.minimum.max(1))
                    .map(|option| option.id)
                    .collect(),
            };
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

fn cast(game: &mut Game, harmonizer: GameObjectId, warped: bool) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if *card == harmonizer && choices.costs().alternative().is_some() == warped)
        })
        .unwrap_or_else(|| panic!("it is castable (warped: {warped})"));
    game.apply(PlayerId::One, action).expect("it is cast");
    settle(game, None);
}

/// Plays a land from hand, which is what the landfall trigger watches for.
fn play_land(game: &mut Game) {
    let land = game
        .build_zone(PlayerId::One, &[cards::FOREST])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = land.id;
    game.players[0].hand.push(land);
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::PlayLand { card, .. } if *card == id))
        .expect("the land can be played");
    game.apply(PlayerId::One, action).expect("it is played");
}

fn permanent(game: &Game, id: GameObjectId) -> &Permanent {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("it is on the battlefield")
}

/// A land drop doubles the power of what it points at.
#[test]
fn a_land_doubles_a_creature() {
    let (mut game, harmonizer, ids) = staged(&[cards::GRIZZLY_BEARS]);
    cast(&mut game, harmonizer, false);
    let bears = ids[0];

    play_land(&mut game);
    settle(&mut game, Some(bears));

    let bears = permanent(&game, bears);
    assert_eq!(game.power(bears), Some(4), "a 2/2 doubled");
    assert_eq!(game.toughness(bears), Some(2), "and toughness untouched");
}

/// Two land drops compound: the second reads the size the first left.
#[test]
fn two_lands_double_twice() {
    let (mut game, harmonizer, ids) = staged(&[cards::GRIZZLY_BEARS]);
    cast(&mut game, harmonizer, false);
    let bears = ids[0];

    play_land(&mut game);
    settle(&mut game, Some(bears));
    game.players[0].lands_played_this_turn = 0;
    play_land(&mut game);
    settle(&mut game, Some(bears));

    assert_eq!(
        game.power(permanent(&game, bears)),
        Some(8),
        "two doublings, not two plus-twos",
    );
}

/// It can point at itself, which is what a 4/4 attacking off a land drop
/// means.
#[test]
fn it_can_double_its_own_power() {
    let (mut game, harmonizer, _) = staged(&[]);
    cast(&mut game, harmonizer, false);
    let body = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::MIGHTFORM_HARMONIZER)
        .expect("it resolved")
        .card
        .id;

    play_land(&mut game);
    settle(&mut game, Some(body));

    assert_eq!(game.power(permanent(&game, body)), Some(8));
}

/// Their land is not one of yours.
#[test]
fn their_land_does_not_trigger_it() {
    let (mut game, harmonizer, _) = staged(&[]);
    cast(&mut game, harmonizer, false);

    game.put_onto_battlefield(PlayerId::Two, cards::MOUNTAIN)
        .expect("cataloged");
    drain_pending(&mut game);

    assert!(game.stack.is_empty(), "nothing triggered");
}

/// Warped, it arrives for three and is exiled at the beginning of the next
/// end step, castable from there afterwards.
#[test]
fn warping_it_lends_the_body_for_a_turn() {
    let (mut game, harmonizer, _) = staged(&[]);

    cast(&mut game, harmonizer, true);
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::MIGHTFORM_HARMONIZER),
        "it arrives for its warp cost",
    );

    game.step = Step::End;
    game.begin_step_triggers();
    settle(&mut game, None);

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.definition != cards::MIGHTFORM_HARMONIZER),
        "and is exiled at the beginning of the next end step",
    );
    assert!(
        game.players[0]
            .exile
            .iter()
            .any(|card| card.definition == cards::MIGHTFORM_HARMONIZER),
        "into exile, to be cast again later",
    );
}

/// Its ruling: "if a creature's power is less than 0 when it's doubled,
/// instead that creature gets -X/-0, where X is how much less than 0 its
/// power is." A Spider shrunk to -1/1 comes out of a land drop at -2/1.
#[test]
fn doubling_a_negative_power_makes_it_worse() {
    let (mut game, harmonizer, ids) = staged(&[cards::GIANT_SPIDER]);
    cast(&mut game, harmonizer, false);
    let spider = ids[0];
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == spider)
        .expect("it is there")
        .set_counters(CounterKind::MinusOneMinusOne, 3);
    let shrunk = permanent(&game, spider);
    assert_eq!(
        (game.power(shrunk), game.toughness(shrunk)),
        (Some(-1), Some(1)),
        "a 2/4 under three counters",
    );

    play_land(&mut game);
    settle(&mut game, Some(spider));

    let doubled = permanent(&game, spider);
    assert_eq!(
        (game.power(doubled), game.toughness(doubled)),
        (Some(-2), Some(1)),
        "doubling minus one is minus two, and the toughness is untouched",
    );
}

/// "Target creature you control": theirs is no target of yours, so with an
/// Angel of theirs the only creature on the other side, the doubling has
/// nowhere to go but the Harmonizer itself.
#[test]
fn it_cannot_double_their_creature() {
    let (mut game, harmonizer, _) = staged(&[]);
    cast(&mut game, harmonizer, false);
    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::SERRA_ANGEL)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    play_land(&mut game);
    settle(&mut game, None);

    let angel = permanent(&game, theirs);
    assert_eq!(
        (game.power(angel), game.toughness(angel)),
        (Some(4), Some(4)),
        "their Angel is the size it was printed",
    );
    let mine = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::MIGHTFORM_HARMONIZER)
        .expect("he is there");
    assert_eq!(
        game.power(mine),
        Some(8),
        "and the doubling landed on the only creature you control",
    );
}
