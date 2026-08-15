//! Reading the toughness of what was sacrificed.
//!
//! The follow-up runs after the permanent is gone, so whichever characteristic
//! it wants is last-known either way -- neither is harder to reach than the
//! other, and the card simply has to say which. Power was the only one
//! authored until these two asked for the other.

use super::*;
use crate::ImplementationStatus;

/// The named source under player one, plus a creature to feed it.
fn board(source: CardDefinitionId, food: CardDefinitionId) -> (Game, GameObjectId, i16) {
    let mut game = ready_game();
    game.turns_started[PlayerId::One.index()] = 5;
    game.step = Step::Upkeep;
    let permanent = creature(10_000, source, PlayerId::One);
    let source_id = permanent.card.id;
    game.battlefield.push(permanent);
    game.battlefield.push(creature(10_001, food, PlayerId::One));
    let life = game.players[PlayerId::One.index()].life;
    (game, source_id, life)
}

/// Activates the source's only ability and answers the sacrifice choice.
fn activate(game: &mut Game, source: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(
            |action| matches!(action, Action::ActivateAbility { source: id, .. } if *id == source),
        )
        .expect("the ability is activatable");
    game.apply(PlayerId::One, action)
        .expect("the ability is activated");
    drain_pending(game);
}

/// Sedge Troll is 2/2, so power and toughness agree and the test would pass
/// either way. Wall of Stone is 0/8, where they disagree by eight.
#[test]
fn diamond_valley_pays_the_toughness_not_the_power() {
    let (mut game, valley, life) = board(cards::DIAMOND_VALLEY, cards::WALL_OF_STONE);
    activate(&mut game, valley);

    assert_eq!(
        game.players[PlayerId::One.index()].life,
        life + 8,
        "eight toughness, not zero power"
    );
}

#[test]
fn life_chisel_reads_it_the_same_way() {
    let (mut game, chisel, life) = board(cards::LIFE_CHISEL, cards::WALL_OF_STONE);
    activate(&mut game, chisel);

    assert_eq!(game.players[PlayerId::One.index()].life, life + 8);
}

/// The creature really is sacrificed, which is what makes the reading
/// last-known.
#[test]
fn the_creature_is_gone_by_the_time_it_is_read() {
    let (mut game, valley, _) = board(cards::DIAMOND_VALLEY, cards::WALL_OF_STONE);
    activate(&mut game, valley);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::WALL_OF_STONE),
        "the Wall was eaten"
    );
}

#[test]
fn every_sacrificed_toughness_identity_reports_complete_coverage() {
    let catalog = poc::catalog().expect("catalog builds");
    for definition in [cards::DIAMOND_VALLEY, cards::LIFE_CHISEL] {
        let card = catalog.get(definition).expect("the card is cataloged");
        assert_eq!(
            card.rules.implementation_status(),
            ImplementationStatus::Complete,
            "{} should be fully executable",
            card.name,
        );
    }
}
