//! Gaea's Cradle: a land worth exactly as much as the board standing beside
//! it.

use super::*;

/// The Cradle untapped and settled in, with `mine` and `theirs` on the
/// battlefield.
fn staged(mine: &[CardDefinitionId], theirs: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let cradle = game
        .put_onto_battlefield(PlayerId::One, cards::GAEAS_CRADLE)
        .expect("cataloged");
    for definition in mine {
        game.put_onto_battlefield(PlayerId::One, *definition)
            .expect("cataloged");
    }
    for definition in theirs {
        game.put_onto_battlefield(PlayerId::Two, *definition)
            .expect("cataloged");
    }
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.empty_mana_pools();
    (game, cradle)
}

/// Taps it and reports the green it made.
fn tap_for_green(game: &mut Game, cradle: GameObjectId) -> u16 {
    let offered = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == cradle)
        .map(|permanent| game.mana_ability_activations(permanent))
        .unwrap_or_default();
    if offered.is_empty() {
        return 0;
    }
    game.apply(
        PlayerId::One,
        Action::ActivateManaAbility {
            source: cradle,
            ability: mana_ability_for(game, cradle, ManaColor::Green),
            color: ManaColor::Green,
            counters_removed: None,
            cost_object: None,
            combination: None,
            triggered_mana: None,
        },
    )
    .expect("it taps for what it is worth");
    game.players[0].mana_pool.green
}

/// One creature, one green; three, three.
#[test]
fn it_makes_a_green_for_each_creature_you_control() {
    for count in [1, 3] {
        let board = vec![cards::GRIZZLY_BEARS; count];
        let (mut game, cradle) = staged(&board, &[]);

        assert_eq!(
            tap_for_green(&mut game, cradle),
            u16::try_from(count).expect("a small board"),
            "a Cradle beside {count} creatures",
        );
    }
}

/// "Each creature you control": theirs are not yours.
#[test]
fn their_creatures_are_not_counted() {
    let (mut game, cradle) = staged(&[cards::GRIZZLY_BEARS], &[cards::SERRA_ANGEL; 3]);

    assert_eq!(
        tap_for_green(&mut game, cradle),
        1,
        "the one bear of yours and none of their Angels",
    );
}

/// An empty board is a land that makes nothing.
#[test]
fn an_empty_board_makes_no_mana() {
    let (mut game, cradle) = staged(&[], &[]);

    assert_eq!(tap_for_green(&mut game, cradle), 0, "nothing to count");
}

/// A creature is a creature however it got there: tokens count, and so does
/// a land that has been animated into one.
#[test]
fn tokens_and_animated_lands_count_too() {
    let (mut game, cradle) = staged(&[cards::JADE_STATUE], &[]);
    game.battlefield.push(token_permanent(
        64_100,
        tokens::creature(&["Bird"], &[ManaColor::White], 1, 1),
        PlayerId::One,
    ));
    assert_eq!(
        tap_for_green(&mut game, cradle),
        1,
        "the token alone, the Statue being no creature yet",
    );

    // Untap it and animate the Statue, which its own ability only allows
    // during combat.
    game.empty_mana_pools();
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == cradle)
    {
        permanent.tapped = false;
    }
    let statue = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::JADE_STATUE)
        .expect("it is there")
        .card
        .id;
    game.step = Step::BeginningOfCombat;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);
    let animate = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == statue),
        )
        .expect("two mana animates it during combat");
    game.apply(PlayerId::One, animate).expect("it activates");
    drain_pending(&mut game);
    game.empty_mana_pools();

    assert_eq!(
        tap_for_green(&mut game, cradle),
        2,
        "the token and the Golem the Statue has become",
    );
}

/// Legendary: a second Cradle is one Cradle too many.
#[test]
fn a_second_cradle_is_binned_by_the_legend_rule() {
    let (mut game, cradle) = staged(&[], &[]);
    game.put_onto_battlefield(PlayerId::One, cards::GAEAS_CRADLE)
        .expect("cataloged");
    drain_pending(&mut game);
    game.check_state_based_actions();
    drain_pending(&mut game);

    assert_eq!(
        game.battlefield
            .iter()
            .filter(|permanent| permanent.card.definition == cards::GAEAS_CRADLE)
            .count(),
        1,
        "one of the two is kept",
    );
    let _ = cradle;
}
