//! Serra Paragon: one card back out of your graveyard every turn, and it
//! leaves for good afterwards.

use super::*;

/// The Paragon on the battlefield since last turn, with `buried` in Player
/// One's graveyard and five mana up.
fn staged(buried: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    let paragon = game
        .put_onto_battlefield(PlayerId::One, cards::SERRA_PARAGON)
        .expect("cataloged");
    for (index, definition) in buried.iter().enumerate() {
        let id = 91_500 + u32::try_from(index).expect("a handful of cards");
        game.players[0]
            .graveyard
            .push(card(id, *definition, PlayerId::One));
    }
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turns_started = [4, 4];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.players[0].life = 20;
    game.players[0].lands_played_this_turn = 0;
    for color in [ManaColor::White, ManaColor::Green, ManaColor::Blue] {
        game.add_unrestricted_mana(PlayerId::One, color, 3);
    }
    (game, paragon)
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
                .map(|option| option.id)
                .take(decision.minimum.max(1))
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

/// The graveyard card of this definition, if it is still there.
fn buried(game: &Game, definition: CardDefinitionId) -> Option<GameObjectId> {
    game.players[0]
        .graveyard
        .iter()
        .find(|card| card.definition == definition)
        .map(|card| card.id)
}

/// The action, if any, that plays or casts `card` out of the graveyard.
fn graveyard_play(game: &Game, card: GameObjectId) -> Option<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::PlayLand { card: id, .. } | Action::CastSpell { card: id, .. } => *id == card,
            _ => false,
        })
}

fn play_from_graveyard(game: &mut Game, definition: CardDefinitionId) {
    let card = buried(game, definition).expect("it is in the graveyard");
    let action = graveyard_play(game, card).expect("the Paragon allows it");
    game.apply(PlayerId::One, action).expect("it is played");
    settle(game);
}

fn on_battlefield(game: &Game, definition: CardDefinitionId) -> Option<GameObjectId> {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == definition)
        .map(|permanent| permanent.card.id)
}

/// A land out of the graveyard, and the land it brings back is marked: it
/// exiles itself when it dies and pays two life for the trouble.
#[test]
fn a_land_comes_back_marked() {
    let (mut game, _paragon) = staged(&[cards::MOUNTAIN]);

    play_from_graveyard(&mut game, cards::MOUNTAIN);

    let mountain = on_battlefield(&game, cards::MOUNTAIN).expect("it is on the battlefield");
    let life = game.players[0].life;
    game.move_permanents_to_graveyard(&[mountain]);
    settle(&mut game);

    assert_eq!(game.players[0].life, life + 2, "two life on the way out");
    assert!(
        buried(&game, cards::MOUNTAIN).is_none(),
        "and it was exiled rather than left in the graveyard",
    );
    assert!(
        game.players[0]
            .exile
            .iter()
            .any(|card| card.definition == cards::MOUNTAIN),
        "which is where it went",
    );
}

/// A cheap permanent spell comes back the same way, with the same clause on
/// it -- the grant rides from the cast through to the permanent.
#[test]
fn a_cheap_permanent_comes_back_marked_too() {
    let (mut game, _paragon) = staged(&[cards::GRIZZLY_BEARS]);

    play_from_graveyard(&mut game, cards::GRIZZLY_BEARS);

    let bears = on_battlefield(&game, cards::GRIZZLY_BEARS).expect("it resolved");
    let life = game.players[0].life;
    game.move_permanents_to_graveyard(&[bears]);
    settle(&mut game);

    assert_eq!(game.players[0].life, life + 2);
    assert!(
        buried(&game, cards::GRIZZLY_BEARS).is_none(),
        "the Bear exiled itself",
    );
}

/// "Once during each of your turns" covers the pair: a land and a spell are
/// not one of each.
#[test]
fn one_play_a_turn_between_them() {
    let (mut game, _paragon) = staged(&[cards::MOUNTAIN, cards::GRIZZLY_BEARS]);

    play_from_graveyard(&mut game, cards::MOUNTAIN);

    let bears = buried(&game, cards::GRIZZLY_BEARS).expect("still there");
    assert!(
        graveyard_play(&game, bears).is_none(),
        "the land spent the turn's one play",
    );
}

/// Mana value three or less, and the Angel is four.
#[test]
fn four_mana_is_too_much() {
    let (game, _paragon) = staged(&[cards::SERRA_ANGEL]);
    let angel = buried(&game, cards::SERRA_ANGEL).expect("it is buried");

    assert!(
        graveyard_play(&game, angel).is_none(),
        "a four-drop is past what the clause names",
    );
}

/// A permanent spell, so an instant or sorcery is not offered however cheap.
#[test]
fn an_instant_is_not_a_permanent() {
    let (game, _paragon) = staged(&[cards::LIGHTNING_BOLT]);
    let bolt = buried(&game, cards::LIGHTNING_BOLT).expect("it is buried");

    assert!(
        graveyard_play(&game, bolt).is_none(),
        "the clause says permanent spell",
    );
}

/// Only your own turns, which is the other half of "once during each of your
/// turns".
#[test]
fn not_on_their_turn() {
    let (mut game, _paragon) = staged(&[cards::MOUNTAIN]);
    game.active_player = PlayerId::Two;
    let mountain = buried(&game, cards::MOUNTAIN).expect("it is buried");

    assert!(
        graveyard_play(&game, mountain).is_none(),
        "their turn is not one of yours",
    );
}

/// The mark is part of the game state, so a checkpoint has to carry it: a
/// rebuilt game exiles the Bear and pays the two life just the same.
#[test]
fn the_mark_survives_a_checkpoint() {
    let (mut game, _paragon) = staged(&[cards::GRIZZLY_BEARS]);
    play_from_graveyard(&mut game, cards::GRIZZLY_BEARS);
    // The hidden half of the payload below carries no libraries, so there
    // must be none to carry.
    game.players[0].library.clear();
    game.players[1].library.clear();

    let viewer = game.priority;
    let observation = game.observe(viewer);
    let actions = crate::protocol::protocol_actions(&observation);
    let wire = crate::protocol::observation_json_for_format(
        &game.catalog,
        game.format,
        &observation,
        false,
        &actions,
    );
    let hidden = serde_json::json!({
        "hands": {"p1": [], "p2": []},
        "libraries": {"p1": [], "p2": []},
        "outsideGame": {"p1": [], "p2": []},
    });
    let mut rebuilt = Game::from_observation_checkpoint(
        game.catalog.clone(),
        game.format,
        &wire,
        &hidden,
        91_507,
    )
    .expect("a marked permanent reconstructs");

    let bears = on_battlefield(&rebuilt, cards::GRIZZLY_BEARS).expect("it is still there");
    let life = rebuilt.players[0].life;
    rebuilt.move_permanents_to_graveyard(&[bears]);
    settle(&mut rebuilt);

    assert_eq!(rebuilt.players[0].life, life + 2, "the clause came along");
    assert!(
        rebuilt.players[0]
            .exile
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS),
        "and it exiled itself",
    );
}

/// The clause outlives the Paragon: a permanent already marked keeps the
/// mark after the Angel that granted it is gone.
#[test]
fn the_mark_outlives_the_paragon() {
    let (mut game, paragon) = staged(&[cards::GRIZZLY_BEARS]);
    play_from_graveyard(&mut game, cards::GRIZZLY_BEARS);
    let bears = on_battlefield(&game, cards::GRIZZLY_BEARS).expect("it resolved");

    game.move_permanents_to_graveyard(&[paragon]);
    settle(&mut game);
    let life = game.players[0].life;
    game.move_permanents_to_graveyard(&[bears]);
    settle(&mut game);

    assert_eq!(game.players[0].life, life + 2, "still two life");
    assert!(
        buried(&game, cards::GRIZZLY_BEARS).is_none(),
        "and still exiled",
    );
}

/// The land half is an ordinary land play, so it costs the turn's land drop
/// as well as the Paragon's one play. Spending the drop elsewhere closes the
/// land half and leaves the spell half open, which is the difference between
/// the two limits.
#[test]
fn the_land_half_also_costs_your_land_drop() {
    let (mut game, _paragon) = staged(&[cards::MOUNTAIN, cards::GRIZZLY_BEARS]);
    game.players[0].lands_played_this_turn = 1;

    let mountain = buried(&game, cards::MOUNTAIN).expect("it is buried");
    assert!(
        graveyard_play(&game, mountain).is_none(),
        "this turn's land has already been played",
    );

    let bears = buried(&game, cards::GRIZZLY_BEARS).expect("it is buried too");
    assert!(
        graveyard_play(&game, bears).is_some(),
        "but the Paragon's own once-a-turn is untouched",
    );

    play_from_graveyard(&mut game, cards::GRIZZLY_BEARS);
    assert!(
        on_battlefield(&game, cards::GRIZZLY_BEARS).is_some(),
        "and the spell half is still there to be spent",
    );
}
