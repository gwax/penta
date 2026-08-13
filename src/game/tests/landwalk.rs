//! Landwalk as one keyword parameterized by land type.
//!
//! CR 702.14 is a single rule: the creature cannot be blocked as long as the
//! defending player controls a land of the named type. The engine used to
//! carry Mountainwalk and Forestwalk as separate keywords with the blocking
//! rule spelled out once per variant, which is why the other three could not
//! be printed. These tests drive the rule through the blocker list a seat is
//! actually offered.

use super::*;
use crate::ImplementationStatus;
use crate::card::BasicLandType;

fn walk_game(walker: CardDefinitionId, defender_land: CardDefinitionId) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.step = Step::DeclareBlockers;
    let mut attacker = creature(10_000, walker, PlayerId::One);
    attacker.attacking = true;
    attacker.attack_defender = Some(AttackDefender::Player(PlayerId::Two));
    let attacker_id = attacker.card.id;
    game.battlefield.push(attacker);
    // Something that could block if the walk did not apply.
    game.battlefield
        .push(creature(10_001, cards::SAVANNAH_LIONS, PlayerId::Two));
    game.battlefield
        .push(creature(10_002, defender_land, PlayerId::Two));
    (game, attacker_id)
}

fn can_be_blocked(game: &Game, attacker: GameObjectId) -> bool {
    game.legal_actions(PlayerId::Two).iter().any(
        |action| matches!(action, Action::DeclareBlocker { attacker: a, .. } if *a == attacker),
    )
}

/// Each printed variant reads its own land type, and only that type.
#[test]
fn every_printed_landwalk_variant_is_stopped_only_by_its_own_land() {
    for (walker, matching, other) in [
        (cards::BOG_WRAITH, cards::SWAMP, cards::ISLAND),
        (cards::RIGHTEOUS_AVENGERS, cards::PLAINS, cards::SWAMP),
        (cards::DEVOURING_DEEP, cards::ISLAND, cards::MOUNTAIN),
        (cards::SEGOVIAN_LEVIATHAN, cards::ISLAND, cards::FOREST),
        (cards::LOST_SOUL, cards::SWAMP, cards::PLAINS),
        (cards::MARSH_GOBLINS, cards::SWAMP, cards::FOREST),
    ] {
        game_declares_blockers_only_without_the_land(walker, matching, other);
    }
}

fn game_declares_blockers_only_without_the_land(
    walker: CardDefinitionId,
    matching: CardDefinitionId,
    other: CardDefinitionId,
) {
    let (blocked, attacker) = walk_game(walker, other);
    assert!(
        can_be_blocked(&blocked, attacker),
        "an unrelated land should not turn on landwalk",
    );

    let (unblockable, attacker) = walk_game(walker, matching);
    assert!(
        !can_be_blocked(&unblockable, attacker),
        "the named land should make the attacker unblockable",
    );
}

/// The rule reads the land's current types rather than its printed name, so a
/// dual land turns on the walk that matches either half.
#[test]
fn landwalk_reads_effective_land_types_rather_than_card_names() {
    let (mut game, attacker) = walk_game(cards::BOG_WRAITH, cards::MOUNTAIN);
    assert!(can_be_blocked(&game, attacker));

    // Badlands is a Swamp Mountain, so the same board now stops blocking.
    game.battlefield
        .retain(|permanent| permanent.card.definition != cards::MOUNTAIN);
    game.battlefield
        .push(creature(10_003, cards::BADLANDS, PlayerId::Two));
    assert!(
        !can_be_blocked(&game, attacker),
        "a dual land counts as both of its types"
    );
}

/// One creature can carry more than one landwalk, and any single match is
/// enough. The old shape could only express this by repeating the rule.
#[test]
fn several_landwalks_on_one_creature_each_stand_on_their_own() {
    let mut game = ready_game();
    game.step = Step::DeclareBlockers;
    let mut attacker = creature(10_000, cards::BOG_WRAITH, PlayerId::One);
    attacker.attacking = true;
    attacker.attack_defender = Some(AttackDefender::Player(PlayerId::Two));
    attacker
        .temporary_keywords
        .push(KeywordAbility::Landwalk(BasicLandType::Island));
    let attacker_id = attacker.card.id;
    game.battlefield.push(attacker);
    game.battlefield
        .push(creature(10_001, cards::SAVANNAH_LIONS, PlayerId::Two));
    game.battlefield
        .push(creature(10_002, cards::ISLAND, PlayerId::Two));

    assert!(
        !can_be_blocked(&game, attacker_id),
        "the granted islandwalk applies even though the printed walk does not"
    );
}

/// A lord grants the walk to everything it names, and the grant behaves like
/// a printed one: the same blocking rule reads it.
#[test]
fn a_granted_landwalk_makes_the_recipient_unblockable() {
    let mut game = ready_game();
    game.step = Step::DeclareBlockers;
    game.battlefield
        .push(creature(10_000, cards::LORD_OF_ATLANTIS, PlayerId::One));
    let mut merfolk = creature(10_001, cards::LORD_OF_ATLANTIS, PlayerId::One);
    merfolk.attacking = true;
    merfolk.attack_defender = Some(AttackDefender::Player(PlayerId::Two));
    let merfolk_id = merfolk.card.id;
    game.battlefield.push(merfolk);
    game.battlefield
        .push(creature(10_002, cards::SAVANNAH_LIONS, PlayerId::Two));

    game.battlefield
        .push(creature(10_003, cards::MOUNTAIN, PlayerId::Two));
    assert!(
        can_be_blocked(&game, merfolk_id),
        "without an Island the granted islandwalk does nothing"
    );

    game.battlefield
        .retain(|permanent| permanent.card.definition != cards::MOUNTAIN);
    game.battlefield
        .push(creature(10_004, cards::ISLAND, PlayerId::Two));
    assert!(
        !can_be_blocked(&game, merfolk_id),
        "the other Merfolk has islandwalk from the lord"
    );
}

/// An Aura grants the walk to what it enchants, which is the same grant
/// mechanism read through an attachment rather than a lord's query.
#[test]
fn an_aura_grants_landwalk_to_the_creature_it_enchants() {
    let mut game = ready_game();
    game.step = Step::DeclareBlockers;
    let mut attacker = creature(10_000, cards::SAVANNAH_LIONS, PlayerId::One);
    attacker.attacking = true;
    attacker.attack_defender = Some(AttackDefender::Player(PlayerId::Two));
    let attacker_id = attacker.card.id;
    game.battlefield.push(attacker);
    game.battlefield
        .push(creature(10_001, cards::SAVANNAH_LIONS, PlayerId::Two));
    game.battlefield
        .push(creature(10_002, cards::ISLAND, PlayerId::Two));
    assert!(
        can_be_blocked(&game, attacker_id),
        "the unenchanted attacker is blockable"
    );

    let mut oil = creature(10_003, cards::FISHLIVER_OIL, PlayerId::One);
    oil.attached_to = Some(attacker_id);
    game.battlefield.push(oil);
    assert!(
        !can_be_blocked(&game, attacker_id),
        "the Aura's islandwalk applies to the creature it enchants"
    );
}

#[test]
fn every_newly_unblocked_walker_reports_complete_coverage() {
    let catalog = poc::catalog().expect("catalog builds");
    for definition in [
        cards::BOG_WRAITH,
        cards::RIGHTEOUS_AVENGERS,
        cards::DEVOURING_DEEP,
        cards::SEGOVIAN_LEVIATHAN,
        cards::LOST_SOUL,
        cards::MARSH_GOBLINS,
        cards::LORD_OF_ATLANTIS,
        cards::FISHLIVER_OIL,
    ] {
        let card = catalog.get(definition).expect("the card is cataloged");
        assert_eq!(
            card.rules.implementation_status(),
            ImplementationStatus::Complete,
            "{} should be fully executable",
            card.name,
        );
    }
}
