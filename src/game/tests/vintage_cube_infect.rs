//! Blightsteel Colossus: infect, and a card that will not stay dead.

use super::*;

/// Infect damage to a player is poison rather than life.
#[test]
fn infect_damage_to_a_player_is_poison() {
    let mut game = ready_game();
    game.battlefield.clear();
    let colossus = creature(95_000, cards::BLIGHTSTEEL_COLOSSUS, PlayerId::One);
    let colossus_id = colossus.card.id;
    game.battlefield.push(colossus);
    let life = game.players[1].life;

    game.damage_target_from_kind(
        Some(colossus_id),
        Some(Target::Player(PlayerId::Two)),
        11,
        true,
    );
    drain_pending(&mut game);

    assert_eq!(game.players[1].life, life, "no life is lost");
    assert_eq!(
        game.players[1].counters.count(CounterKind::Poison),
        11,
        "eleven poison instead, which is past the ten it takes to lose",
    );
    game.check_state_based_actions();
    assert_eq!(
        game.result(),
        Some(GameResult::Winner {
            winner: PlayerId::One,
            reason: WinReason::OpponentPoisoned,
        }),
        "and the tenth counter is a state-based loss",
    );
}

/// Infect changes what damage does to creatures and players, not to
/// planeswalkers: those still lose that much loyalty.
#[test]
fn infect_damage_to_a_planeswalker_is_still_loyalty() {
    let mut game = ready_game();
    game.battlefield.clear();
    let colossus = creature(95_040, cards::BLIGHTSTEEL_COLOSSUS, PlayerId::One);
    let colossus_id = colossus.card.id;
    game.battlefield.push(colossus);
    let jace = game
        .put_onto_battlefield(PlayerId::Two, cards::JACE_MEMORY_ADEPT)
        .expect("cataloged");
    drain_pending(&mut game);
    let starting = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == jace)
        .expect("he is there")
        .counters(CounterKind::Loyalty);

    game.damage_target_from_kind(Some(colossus_id), Some(Target::Permanent(jace)), 2, true);
    drain_pending(&mut game);

    let walker = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == jace)
        .expect("five loyalty survives two");
    assert_eq!(
        walker.counters(CounterKind::Loyalty),
        starting - 2,
        "the loyalty is what the damage removed",
    );
    assert_eq!(
        walker.counters(CounterKind::MinusOneMinusOne),
        0,
        "and infect put no counters on him",
    );
}

/// Infect damage to a creature is -1/-1 counters rather than damage marks.
#[test]
fn infect_damage_to_a_creature_is_counters() {
    let mut game = ready_game();
    game.battlefield.clear();
    let colossus = creature(95_010, cards::BLIGHTSTEEL_COLOSSUS, PlayerId::One);
    let colossus_id = colossus.card.id;
    game.battlefield.push(colossus);
    let angel = creature(95_011, cards::SERRA_ANGEL, PlayerId::Two);
    let angel_id = angel.card.id;
    game.battlefield.push(angel);

    game.damage_target_from_kind(
        Some(colossus_id),
        Some(Target::Permanent(angel_id)),
        2,
        true,
    );
    drain_pending(&mut game);

    let angel = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == angel_id)
        .expect("a 4/4 survives two");
    assert_eq!(angel.counters(CounterKind::MinusOneMinusOne), 2);
    assert_eq!(angel.damage, 0, "and takes no damage at all");
    assert_eq!(game.power(angel), Some(2), "it is permanently smaller");
    assert_eq!(game.toughness(angel), Some(2));
}

/// Destroying it puts it back in the library rather than the graveyard, and
/// nothing is left behind to reanimate.
#[test]
fn the_colossus_shuffles_itself_back_instead_of_dying() {
    let mut game = ready_game();
    game.battlefield.clear();
    let colossus = creature(95_020, cards::BLIGHTSTEEL_COLOSSUS, PlayerId::One);
    let colossus_id = colossus.card.id;
    game.battlefield.push(colossus);
    game.players[0].library.clear();
    let before = game.players[0].library.len();

    // Indestructible stops destruction, so this is the sacrifice route --
    // which the replacement answers all the same.
    game.move_permanents_to_graveyard(&[colossus_id]);
    drain_pending(&mut game);

    assert!(
        game.players[0].graveyard.is_empty(),
        "it never reaches the graveyard",
    );
    assert_eq!(
        game.players[0].library.len(),
        before + 1,
        "and goes back into the library instead",
    );
    assert!(
        game.players[0]
            .library
            .iter()
            .any(|card| card.definition == cards::BLIGHTSTEEL_COLOSSUS),
        "as itself",
    );
}

/// "From anywhere" means from anywhere: a discarded Colossus goes back too,
/// and it is shuffled in rather than left on top.
#[test]
fn a_discarded_colossus_goes_back_and_is_shuffled_in() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].library.clear();
    for id in 95_040..95_060 {
        game.players[0]
            .library
            .push(card(id, cards::GRIZZLY_BEARS, PlayerId::One));
    }
    let colossus = card(95_030, cards::BLIGHTSTEEL_COLOSSUS, PlayerId::One);
    let colossus_id = colossus.id;
    game.players[0].hand.push(colossus);

    // Discarded from hand rather than dying, which is a graveyard move from
    // a different zone entirely.
    game.discard_cards(PlayerId::One, &[colossus_id]);
    drain_pending(&mut game);

    assert!(
        game.players[0].graveyard.is_empty(),
        "a discard does not put it in the graveyard either",
    );
    let position = game.players[0]
        .library
        .iter()
        .position(|card| card.definition == cards::BLIGHTSTEEL_COLOSSUS)
        .expect("it goes back into the library from hand as well");
    assert_ne!(
        position,
        game.players[0].library.len() - 1,
        "shuffled in rather than left on top, where it would just be redrawn",
    );
}

/// "The -1/-1 counters remain on the creature indefinitely. They're not
/// removed ... at end of turn." Ordinary damage wears off in the cleanup
/// step; what infect does instead is the reason it is worth the drawback.
#[test]
fn the_counters_outlast_the_turn_that_marked_damage_does_not() {
    let mut game = ready_game();
    game.battlefield.clear();
    let colossus = creature(95_060, cards::BLIGHTSTEEL_COLOSSUS, PlayerId::One);
    let colossus_id = colossus.card.id;
    game.battlefield.push(colossus);
    let bears = creature(95_061, cards::GRIZZLY_BEARS, PlayerId::One);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    let poisoned = creature(95_062, cards::SERRA_ANGEL, PlayerId::Two);
    let poisoned_id = poisoned.card.id;
    game.battlefield.push(poisoned);
    let bruised = creature(95_063, cards::SERRA_ANGEL, PlayerId::Two);
    let bruised_id = bruised.card.id;
    game.battlefield.push(bruised);

    // One Angel takes two from the Colossus, the other two from the Bears.
    game.damage_target_from_kind(
        Some(colossus_id),
        Some(Target::Permanent(poisoned_id)),
        2,
        true,
    );
    game.damage_target_from_kind(
        Some(bears_id),
        Some(Target::Permanent(bruised_id)),
        2,
        false,
    );
    drain_pending(&mut game);
    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == bruised_id)
            .expect("it survived")
            .damage,
        2,
        "the ordinary two is marked damage",
    );

    game.finish_cleanup();

    let poisoned = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == poisoned_id)
        .expect("still a 2/2");
    assert_eq!(
        poisoned.counters(CounterKind::MinusOneMinusOne),
        2,
        "the counters are still there a turn later",
    );
    assert_eq!(game.toughness(poisoned), Some(2), "and still shrink it");
    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == bruised_id)
            .expect("still a 4/4")
            .damage,
        0,
        "while the marked damage is gone, which is the whole difference",
    );
}

/// "From anywhere" reaches the stack: a Colossus answered on its way in is
/// shuffled back rather than left in the graveyard for a second attempt at
/// cheating it out.
#[test]
fn a_countered_colossus_goes_back_into_the_library() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[1].hand.clear();
    game.players[0].library.clear();
    for id in 95_200..95_212 {
        game.players[0]
            .library
            .push(card(id, cards::GRIZZLY_BEARS, PlayerId::One));
    }
    let colossus = card(95_220, cards::BLIGHTSTEEL_COLOSSUS, PlayerId::One);
    let colossus_id = colossus.id;
    game.players[0].hand.push(colossus);
    game.players[1]
        .hand
        .push(card(95_221, cards::COUNTERSPELL, PlayerId::Two));
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 12);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Blue, 2);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == colossus_id))
        .expect("twelve mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    game.priority = PlayerId::Two;
    let counter = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, .. } if *card == CardInstanceId(95_221))
        })
        .expect("two blue answers it");
    game.apply(PlayerId::Two, counter).expect("it is cast");
    drain_pending(&mut game);

    assert!(
        game.players[0]
            .graveyard
            .iter()
            .all(|card| card.definition != cards::BLIGHTSTEEL_COLOSSUS),
        "a countered Colossus is not a Colossus in the graveyard",
    );
    let position = game.players[0]
        .library
        .iter()
        .position(|card| card.definition == cards::BLIGHTSTEEL_COLOSSUS)
        .expect("it went back into the library from the stack");
    assert_ne!(
        position,
        game.players[0].library.len() - 1,
        "shuffled in rather than sitting on top",
    );
}

/// Milled from the library it never leaves it: the replacement watches the
/// library too, so a Colossus turned over by a Brain Freeze comes straight
/// back rather than filling the graveyard it was aimed at.
#[test]
fn a_milled_colossus_never_reaches_the_graveyard() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[1].library.clear();
    for id in 95_300..95_310 {
        game.players[1]
            .library
            .push(card(id, cards::GRIZZLY_BEARS, PlayerId::Two));
    }
    game.players[1]
        .library
        .push(card(95_320, cards::BLIGHTSTEEL_COLOSSUS, PlayerId::Two));
    let freeze = card(95_330, cards::BRAIN_FREEZE, PlayerId::One);
    let freeze_id = freeze.id;
    game.players[0].hand.push(freeze);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == freeze_id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Player(PlayerId::Two))
            }
            _ => false,
        })
        .expect("two mana points it at them");
    game.apply(PlayerId::One, cast).expect("it is cast");
    pass_until_decision(&mut game);
    drain_pending(&mut game);

    assert_eq!(
        game.players[1].graveyard.len(),
        2,
        "the two Bears under it were milled",
    );
    assert!(
        game.players[1]
            .graveyard
            .iter()
            .all(|card| card.definition != cards::BLIGHTSTEEL_COLOSSUS),
        "and the Colossus was not",
    );
    assert_eq!(
        game.players[1]
            .library
            .iter()
            .filter(|card| card.definition == cards::BLIGHTSTEEL_COLOSSUS)
            .count(),
        1,
        "it is back in the library it was turned over from",
    );
}

/// Trample and infect together: a Colossus held by a 2/2 assigns two to the
/// blocker and the other nine over the top, and every point of it is poison.
#[test]
fn trample_over_a_blocker_is_poison_as_well() {
    let mut game = ready_game();
    game.battlefield.clear();
    let colossus = creature(95_400, cards::BLIGHTSTEEL_COLOSSUS, PlayerId::One);
    let colossus_id = colossus.card.id;
    game.battlefield.push(colossus);
    let chump = creature(95_401, cards::GRIZZLY_BEARS, PlayerId::Two);
    let chump_id = chump.card.id;
    game.battlefield.push(chump);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.priority = PlayerId::One;
    let life = game.players[1].life;

    game.declare_attacker(colossus_id, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    drain_pending(&mut game);
    game.declare_blocker(chump_id, colossus_id);
    game.step = Step::CombatDamage;
    game.deal_combat_damage();
    drain_pending(&mut game);

    assert_eq!(game.players[1].life, life, "no life is lost to it");
    assert_eq!(
        game.players[1].counters.count(CounterKind::Poison),
        9,
        "two to the blocker and nine over the top, as poison",
    );
    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != chump_id),
        "and the blocker took two -1/-1 counters it could not survive",
    );
}
