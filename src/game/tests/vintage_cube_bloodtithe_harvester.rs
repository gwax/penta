//! Bloodtithe Harvester: a two-mana body that leaves a card behind, and the
//! removal it turns into once the attacking is over.

use super::*;

/// The Harvester on the battlefield since last turn, with `extra` further
/// Blood tokens beside the one it made, and a 2/2 across the table.
fn staged(extra: usize) -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let harvester = game
        .put_onto_battlefield(PlayerId::One, cards::BLOODTITHE_HARVESTER)
        .expect("cataloged");
    drain_pending(&mut game);
    for _ in 0..extra {
        game.create_token(PlayerId::One, tokens::blood());
    }
    let bears = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turns_started = [4, 4];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, harvester, bears)
}

fn bloods(game: &Game) -> usize {
    game.battlefield
        .iter()
        .filter(|permanent| is_token_with(permanent, tokens::blood()))
        .count()
}

/// The one action, if any, that fires the Harvester's removal at `victim`.
fn removal(game: &Game, harvester: GameObjectId, victim: GameObjectId) -> Option<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } => {
                *source == harvester
                    && targets.iter().any(|selection| {
                        selection
                            .targets()
                            .iter()
                            .any(|target| matches!(target, Target::Permanent(id) if *id == victim))
                    })
            }
            _ => false,
        })
}

fn on_battlefield(game: &Game, id: GameObjectId) -> bool {
    game.battlefield
        .iter()
        .any(|permanent| permanent.card.id == id)
}

/// Arriving makes the Blood, which is the half of the card that survives it.
#[test]
fn it_brings_a_blood_token_with_it() {
    let (game, _harvester, _bears) = staged(0);

    assert_eq!(bloods(&game), 1, "one Blood from the arrival");
}

/// One Blood is -2/-2, which is exactly a Bear.
#[test]
fn one_blood_kills_a_two_two() {
    let (mut game, harvester, bears) = staged(0);

    let action = removal(&game, harvester, bears).expect("the removal is offered");
    game.apply(PlayerId::One, action).expect("it activates");
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert!(!on_battlefield(&game, bears), "the Bear died");
    assert!(
        !on_battlefield(&game, harvester),
        "and the Harvester paid for it with itself",
    );
    assert_eq!(bloods(&game), 1, "the Blood is untouched by any of it");
}

/// The count is doubled, so a second Blood is the difference between
/// wounding a 4/4 and killing it.
#[test]
fn a_second_blood_takes_four() {
    for (extra, survives) in [(0, true), (1, false)] {
        let (mut game, harvester, _bears) = staged(extra);
        let angel = game
            .put_onto_battlefield(PlayerId::Two, cards::SERRA_ANGEL)
            .expect("cataloged");
        drain_pending(&mut game);

        let action = removal(&game, harvester, angel).expect("the removal is offered");
        game.apply(PlayerId::One, action).expect("it activates");
        drain_pending(&mut game);
        game.check_state_based_actions();

        match game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == angel)
        {
            Some(permanent) => {
                assert!(survives, "two Blood should have killed it");
                assert_eq!(game.power(permanent), Some(2), "one Blood is -2/-2");
                assert_eq!(game.toughness(permanent), Some(2));
            }
            None => assert!(!survives, "one Blood should have left a 2/2"),
        }
    }
}

/// The number is read as the ability resolves, so a Blood spent for a card
/// beforehand is a Blood that no longer counts.
#[test]
fn spending_the_blood_first_costs_the_removal_its_size() {
    let (mut game, harvester, bears) = staged(0);
    let blood = game
        .battlefield
        .iter()
        .find(|permanent| is_token_with(permanent, tokens::blood()))
        .expect("the arrival made one")
        .card
        .id;
    game.players[0]
        .hand
        .push(card(96_000, cards::FOREST, PlayerId::One));
    game.players[0].mana_pool.colorless = 1;
    let loot = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == blood))
        .expect("the Blood can be cashed in");
    game.apply(PlayerId::One, loot).expect("it activates");
    drain_pending(&mut game);
    assert_eq!(bloods(&game), 0, "the token sacrificed itself");
    // Draining the draw ran the rest of the turn out, so put the game back
    // in the window the sorcery-speed clause asks for.
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    let action = removal(&game, harvester, bears).expect("the removal is still offered");
    game.apply(PlayerId::One, action).expect("it activates");
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert!(
        on_battlefield(&game, bears),
        "no Blood is -0/-0, and the Harvester died for nothing",
    );
}

/// "Activate only as a sorcery": not on their turn, and not with something
/// already on the stack.
#[test]
fn the_removal_waits_for_a_sorcery_window() {
    let (mut game, harvester, bears) = staged(0);
    game.active_player = PlayerId::Two;
    game.step = Step::PrecombatMain;
    assert!(
        removal(&game, harvester, bears).is_none(),
        "their main phase is not your sorcery window",
    );

    game.active_player = PlayerId::One;
    game.step = Step::Upkeep;
    assert!(
        removal(&game, harvester, bears).is_none(),
        "and neither is your upkeep",
    );

    game.step = Step::PrecombatMain;
    assert!(
        removal(&game, harvester, bears).is_some(),
        "your own main phase with an empty stack is",
    );
}

/// The tap in the cost still means what it always does: the turn it arrives,
/// it can only be a creature.
#[test]
fn a_fresh_harvester_cannot_activate() {
    let (mut game, harvester, bears) = staged(0);
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == harvester)
        .expect("he is there")
        .entered_controller_turn = game.turns_started[0];

    assert!(
        removal(&game, harvester, bears).is_none(),
        "summoning sickness stops the tap",
    );
}

/// "The value of X is determined only once, as the ability resolves." Not as
/// it is activated: a Blood cashed in while the removal waits on the stack
/// is a Blood the removal no longer counts.
#[test]
fn a_blood_spent_in_response_shrinks_the_removal() {
    let (mut game, harvester, _bears) = staged(1);
    let angel = game
        .put_onto_battlefield(PlayerId::Two, cards::SERRA_ANGEL)
        .expect("cataloged");
    drain_pending(&mut game);
    assert_eq!(bloods(&game), 2, "two Blood is -4/-4, which kills a 4/4");

    let action = removal(&game, harvester, angel).expect("the removal is offered");
    game.apply(PlayerId::One, action).expect("it activates");

    // In response, one of the two is cashed in for a card.
    let blood = game
        .battlefield
        .iter()
        .find(|permanent| is_token_with(permanent, tokens::blood()))
        .expect("one is still there")
        .card
        .id;
    game.players[0]
        .hand
        .push(card(96_100, cards::FOREST, PlayerId::One));
    game.players[0].mana_pool.colorless = 1;
    let loot = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == blood))
        .expect("the Blood can be cashed in while the removal waits");
    game.apply(PlayerId::One, loot).expect("it activates");
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert_eq!(bloods(&game), 1, "one Blood is left when it resolves");
    let survivor = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == angel)
        .expect("minus two is not lethal to a 4/4");
    assert_eq!(game.power(survivor), Some(2));
    assert_eq!(game.toughness(survivor), Some(2));
}
