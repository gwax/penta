//! Cecil, Dark Knight // Cecil, Redeemed Paladin: a one-mana deathtoucher
//! who turns your own life total into the cost of turning him over.
//!
//! The transform threshold and the Paladin's "other attacking creatures"
//! clause are pinned in `vintage_cube_creatures`. What is here is the ruling
//! about him dying as he deals his damage, and the faces either side of it.

use super::*;

/// Cecil on the battlefield at `life`, out since last turn.
fn staged(life: i16) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let cecil = game
        .put_onto_battlefield(PlayerId::One, cards::CECIL_DARK_KNIGHT)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.players[0].life = life;
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, cecil)
}

fn face_of(game: &Game, cecil: GameObjectId) -> Option<CardPartId> {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == cecil)
        .map(|permanent| permanent.presented)
}

/// "If Cecil, Dark Knight is dealt lethal damage at the same time it deals
/// damage, its last ability will still trigger. You'll still lose life, but
/// you won't untap or transform Cecil because it won't be on the battlefield
/// anymore." A Grave Titan blocking him settles all three halves at once.
#[test]
fn dying_as_he_strikes_costs_the_life_and_turns_nothing_over() {
    let (mut game, cecil) = staged(5);
    let titan = game
        .put_onto_battlefield(PlayerId::Two, cards::GRAVE_TITAN)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.priority = PlayerId::One;

    game.declare_attacker(cecil, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    drain_pending(&mut game);
    game.step = Step::DeclareBlockers;
    game.blockers_declared = false;
    game.declare_blocker(titan, cecil);
    game.finish_declaring_blockers();
    game.step = Step::CombatDamage;
    game.deal_combat_damage();
    drain_pending(&mut game);
    game.check_state_based_actions();
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != cecil),
        "six from the Titan is lethal to a 2/3",
    );
    assert_eq!(
        game.players[0].life, 3,
        "and the two he dealt is still lost, dead or not",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::CECIL_DARK_KNIGHT),
        "he is in the graveyard rather than turned over",
    );
}

/// Deathtouch is the front face's, and it is what makes a 2/3 trade with
/// anything: the Titan that killed him died of two.
#[test]
fn his_deathtouch_kills_what_blocks_him() {
    let (mut game, cecil) = staged(20);
    let titan = game
        .put_onto_battlefield(PlayerId::Two, cards::GRAVE_TITAN)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.priority = PlayerId::One;

    game.declare_attacker(cecil, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    drain_pending(&mut game);
    game.step = Step::DeclareBlockers;
    game.blockers_declared = false;
    game.declare_blocker(titan, cecil);
    game.finish_declaring_blockers();
    game.step = Step::CombatDamage;
    game.deal_combat_damage();
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != titan),
        "two points of deathtouch is lethal to six toughness",
    );
}

/// "The mana value of a nonmodal double-faced card is the mana value of its
/// front face, no matter which face is up." The Paladin prints no cost of
/// his own and is worth the Dark Knight's one.
#[test]
fn the_paladin_is_worth_the_dark_knights_one() {
    let (mut game, cecil) = staged(5);
    assert_eq!(
        game.permanent_mana_value(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == cecil)
                .expect("he is out"),
        ),
        1,
        "{{B}} is one",
    );

    // His own damage halves his controller's life, which turns him over.
    game.damage_target_from(Some(cecil), Some(Target::Player(PlayerId::Two)), 1);
    drain_pending(&mut game);
    assert_eq!(
        face_of(&game, cecil),
        Some(CardPartId(1)),
        "the Paladin side is up",
    );

    assert_eq!(
        game.permanent_mana_value(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == cecil)
                .expect("he is still out"),
        ),
        1,
        "and turning him over does not change what he cost",
    );
}

/// "In every zone other than the battlefield, consider only the
/// characteristics of its front face." A Cecil in the graveyard is the Dark
/// Knight, whichever face was up when he died.
#[test]
fn off_the_battlefield_he_is_the_dark_knight() {
    let (mut game, cecil) = staged(5);
    game.damage_target_from(Some(cecil), Some(Target::Player(PlayerId::Two)), 1);
    drain_pending(&mut game);
    assert_eq!(
        face_of(&game, cecil),
        Some(CardPartId(1)),
        "he turned over first",
    );

    game.move_permanents_to_graveyard(&[cecil]);
    game.check_state_based_actions();
    let buried = game.players[0].graveyard.last().expect("he is lying there");

    // The faces differ in colour and nothing else useful: the Dark Knight is
    // black, and the Paladin's indicator is white.
    assert!(
        game.card_object_matches(
            ObjectPredicateDef::Color(ManaColor::Black),
            buried,
            ZoneKind::Graveyard,
            cecil,
        ),
        "the front face's black is what the graveyard reads",
    );
    assert!(
        !game.card_object_matches(
            ObjectPredicateDef::Color(ManaColor::White),
            buried,
            ZoneKind::Graveyard,
            cecil,
        ),
        "and the Paladin's white is no part of him there, however he died",
    );
}
