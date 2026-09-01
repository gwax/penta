//! Field of the Dead: a Zombie for every land drop, once the lands stop
//! sharing names.

use super::*;

/// The Field on the battlefield with `others` beside it, all under Player
/// One and all having been there since last turn.
fn staged(others: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let field = game
        .put_onto_battlefield(PlayerId::One, cards::FIELD_OF_THE_DEAD)
        .expect("cataloged");
    for definition in others {
        game.put_onto_battlefield(PlayerId::One, *definition)
            .expect("cataloged");
    }
    drain_pending(&mut game);
    settle(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turns_started = [8, 8];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, field)
}

fn settle(game: &mut Game) {
    for _ in 0..24 {
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

fn zombies(game: &Game) -> usize {
    game.battlefield
        .iter()
        .filter(|permanent| {
            is_token_with(
                permanent,
                tokens::creature(&["Zombie"], &[ManaColor::Black], 2, 2),
            )
        })
        .count()
}

/// Puts one more land onto the battlefield and lets the trigger settle.
fn land_arrives(game: &mut Game, definition: CardDefinitionId, player: PlayerId) {
    game.put_onto_battlefield(player, definition)
        .expect("cataloged");
    drain_pending(game);
    settle(game);
}

/// Six other names plus the Field is seven, so the seventh land is the one
/// that starts it.
#[test]
fn the_seventh_name_starts_the_zombies() {
    let (mut game, _field) = staged(&[
        cards::PLAINS,
        cards::ISLAND,
        cards::SWAMP,
        cards::MOUNTAIN,
        cards::FOREST,
    ]);
    assert_eq!(zombies(&game), 0, "six names is not seven");

    land_arrives(&mut game, cards::TUNDRA, PlayerId::One);

    assert_eq!(zombies(&game), 1, "the seventh name made one");

    land_arrives(&mut game, cards::TAIGA, PlayerId::One);

    assert_eq!(zombies(&game), 2, "and every land after it makes another");
}

/// Names, not lands: a second Mountain is not a seventh name.
#[test]
fn duplicate_names_do_not_count() {
    let (mut game, _field) = staged(&[
        cards::PLAINS,
        cards::ISLAND,
        cards::SWAMP,
        cards::MOUNTAIN,
        cards::FOREST,
    ]);

    land_arrives(&mut game, cards::MOUNTAIN, PlayerId::One);

    assert_eq!(
        zombies(&game),
        0,
        "six names and two Mountains is still six"
    );

    land_arrives(&mut game, cards::TUNDRA, PlayerId::One);
    assert_eq!(zombies(&game), 1, "the new name is the seventh");
}

/// The condition is checked as the trigger would go on the stack and again
/// as it resolves, so a land that arrives under the other player does not
/// count toward your seven.
#[test]
fn their_lands_are_not_yours() {
    let (mut game, _field) = staged(&[
        cards::PLAINS,
        cards::ISLAND,
        cards::SWAMP,
        cards::MOUNTAIN,
        cards::FOREST,
        cards::TUNDRA,
    ]);
    let before = zombies(&game);

    land_arrives(&mut game, cards::TAIGA, PlayerId::Two);

    assert_eq!(
        zombies(&game),
        before,
        "their land is neither the trigger nor one of the seven",
    );
}

/// The Field itself is one of the names it counts.
#[test]
fn the_field_counts_itself() {
    let (mut game, _field) = staged(&[
        cards::PLAINS,
        cards::ISLAND,
        cards::SWAMP,
        cards::MOUNTAIN,
        cards::FOREST,
    ]);

    land_arrives(&mut game, cards::TUNDRA, PlayerId::One);

    assert_eq!(
        game.battlefield
            .iter()
            .filter(|permanent| permanent.card.definition != ObjectKind::Token)
            .count(),
        7,
        "seven lands, one of them the Field",
    );
    assert_eq!(zombies(&game), 1);
}

/// It comes in tapped and makes colourless, which is what the deck pays for
/// the rest of it.
#[test]
fn it_enters_tapped_and_taps_for_colorless() {
    let mut game = ready_game();
    game.battlefield.clear();
    let field = game
        .put_onto_battlefield(PlayerId::One, cards::FIELD_OF_THE_DEAD)
        .expect("cataloged");
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == field)
            .expect("it is there")
            .tapped,
        "it enters tapped",
    );

    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == field)
        .expect("it is there")
        .tapped = false;
    game.priority = PlayerId::One;
    let mana = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(
            |action| matches!(action, Action::ActivateManaAbility { source, .. } if *source == field),
        )
        .expect("it taps for mana");
    game.apply(PlayerId::One, mana).expect("it activates");

    assert_eq!(game.players[0].mana_pool.colorless, 1);
}

/// "This land or another land you control": a second Field watches the same
/// arrivals the first one does, so each land drop past the seventh name is
/// two Zombies -- and the two Fields are one name between them.
#[test]
fn a_second_field_doubles_every_arrival() {
    let (mut game, _field) = staged(&[
        cards::FIELD_OF_THE_DEAD,
        cards::PLAINS,
        cards::ISLAND,
        cards::SWAMP,
        cards::MOUNTAIN,
        cards::FOREST,
    ]);
    assert_eq!(
        zombies(&game),
        0,
        "two Fields and five basics are six names, not seven",
    );

    land_arrives(&mut game, cards::TUNDRA, PlayerId::One);

    assert_eq!(zombies(&game), 2, "the seventh name is seen by both Fields");

    land_arrives(&mut game, cards::TAIGA, PlayerId::One);

    assert_eq!(zombies(&game), 4, "and so is the eighth");
}

/// "Whenever *this land* or another land you control enters": the Field
/// watches its own arrival, so the Field itself landing as the seventh name
/// pays out at once rather than waiting for the next land drop.
#[test]
fn the_field_arriving_seventh_pays_out_on_itself() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    for definition in [
        cards::PLAINS,
        cards::ISLAND,
        cards::SWAMP,
        cards::MOUNTAIN,
        cards::FOREST,
        cards::TUNDRA,
    ] {
        game.put_onto_battlefield(PlayerId::One, definition)
            .expect("cataloged");
    }
    drain_pending(&mut game);
    settle(&mut game);
    game.turns_started = [8, 8];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    assert_eq!(zombies(&game), 0, "six names and no Field yet");

    land_arrives(&mut game, cards::FIELD_OF_THE_DEAD, PlayerId::One);

    assert_eq!(
        zombies(&game),
        1,
        "the seventh name was the Field, and it counted itself",
    );
}

/// "If you control seven or more lands with different names" is read again
/// as the trigger resolves. A land answered while the trigger waits takes
/// the seventh name away with it, and nothing is created.
#[test]
fn a_land_lost_while_the_trigger_waits_takes_the_zombie_with_it() {
    let (mut game, _field) = staged(&[
        cards::PLAINS,
        cards::ISLAND,
        cards::SWAMP,
        cards::MOUNTAIN,
        cards::FOREST,
    ]);
    let doomed = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::PLAINS)
        .expect("the Plains is out")
        .card
        .id;

    game.put_onto_battlefield(PlayerId::One, cards::TUNDRA)
        .expect("cataloged");
    // Onto the stack but not yet resolved, which is the window the second
    // check of the intervening if belongs to.
    game.finish_rules_procedure();
    assert_eq!(game.stack.len(), 1, "the seventh name put the trigger up");

    game.move_permanents_to_graveyard(&[doomed]);
    game.check_state_based_actions();
    settle(&mut game);

    assert_eq!(
        zombies(&game),
        0,
        "six names by the time it resolved, so nothing was created",
    );
}
