//! Combat requirements: "all creatures able to block this do so".
//!
//! A requirement never beats a restriction, so "able" is read from the same
//! legality that offers a block in the first place. What the requirement does
//! is take the alternatives away: a creature that could block the lured
//! attacker is offered no other seat, and the defending player cannot finish
//! declaring blockers while one of them is still standing free.

use super::*;
use crate::ImplementationStatus;

/// Two attackers for player one, with `lured` carrying the requirement.
fn two_attackers(lured: CardDefinitionId, other: CardDefinitionId) -> (Game, [GameObjectId; 2]) {
    let mut game = ready_game();
    game.step = Step::DeclareBlockers;
    let mut ids = [GameObjectId(0); 2];
    for (index, definition) in [lured, other].into_iter().enumerate() {
        let id = 10_000 + u32::try_from(index).expect("two attackers fit");
        let mut attacker = creature(id, definition, PlayerId::One);
        attacker.attacking = true;
        attacker.attack_defender = Some(AttackDefender::Player(PlayerId::Two));
        ids[index] = attacker.card.id;
        game.battlefield.push(attacker);
    }
    (game, ids)
}

fn add_blocker(game: &mut Game, id: u32, definition: CardDefinitionId) -> GameObjectId {
    let blocker = creature(id, definition, PlayerId::Two);
    let blocker_id = blocker.card.id;
    game.battlefield.push(blocker);
    blocker_id
}

/// Which attackers this creature is currently offered as a blocker for.
fn seats(game: &Game, blocker: GameObjectId) -> Vec<GameObjectId> {
    game.legal_actions(PlayerId::Two)
        .into_iter()
        .filter_map(|action| match action {
            Action::DeclareBlocker {
                blocker: actual,
                attacker,
            } if actual == blocker => Some(attacker),
            _ => None,
        })
        .collect()
}

fn may_finish(game: &Game) -> bool {
    game.legal_actions(PlayerId::Two)
        .iter()
        .any(|action| matches!(action, Action::FinishDeclaringBlockers))
}

/// Marble Priest carries the requirement itself, so nothing has to be
/// attached to it.
#[test]
fn a_wall_is_offered_only_the_creature_it_must_block() {
    let (mut game, [priest, other]) = two_attackers(cards::MARBLE_PRIEST, cards::SEDGE_TROLL);
    let wall = add_blocker(&mut game, 10_010, cards::WALL_OF_STONE);

    assert_eq!(
        seats(&game, wall),
        vec![priest],
        "the requirement takes the other attacker away"
    );
    assert_ne!(priest, other);
}

/// The requirement names Walls, so anything else blocks as it likes.
#[test]
fn a_creature_the_requirement_does_not_name_keeps_every_seat() {
    let (mut game, [priest, other]) = two_attackers(cards::MARBLE_PRIEST, cards::SEDGE_TROLL);
    let lion = add_blocker(&mut game, 10_010, cards::SAVANNAH_LIONS);

    let mut offered = seats(&game, lion);
    offered.sort_unstable();
    let mut expected = vec![priest, other];
    expected.sort_unstable();
    assert_eq!(offered, expected);
}

#[test]
fn the_declaration_cannot_finish_while_a_requirement_is_unmet() {
    let (mut game, [priest, _]) = two_attackers(cards::MARBLE_PRIEST, cards::SEDGE_TROLL);
    let wall = add_blocker(&mut game, 10_010, cards::WALL_OF_STONE);
    assert!(!may_finish(&game), "the Wall is still standing free");

    game.apply(
        PlayerId::Two,
        Action::DeclareBlocker {
            blocker: wall,
            attacker: priest,
        },
    )
    .expect("the required block is legal");

    assert!(may_finish(&game), "the requirement is met");
}

/// A tapped creature is not able to block, so it is not required to, and its
/// controller is free to finish.
#[test]
fn a_creature_that_cannot_block_is_not_required_to() {
    let (mut game, _) = two_attackers(cards::MARBLE_PRIEST, cards::SEDGE_TROLL);
    let wall = add_blocker(&mut game, 10_010, cards::WALL_OF_STONE);
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == wall)
        .expect("the Wall is on the battlefield")
        .tapped = true;

    assert!(seats(&game, wall).is_empty());
    assert!(may_finish(&game));
}

/// The Walls that are forced in take nothing for it.
#[test]
fn the_priest_takes_no_combat_damage_from_walls() {
    let (mut game, [priest, _]) = two_attackers(cards::MARBLE_PRIEST, cards::SEDGE_TROLL);
    let wall = add_blocker(&mut game, 10_010, cards::WALL_OF_STONE);
    game.apply(
        PlayerId::Two,
        Action::DeclareBlocker {
            blocker: wall,
            attacker: priest,
        },
    )
    .expect("the required block is legal");
    game.deal_combat_damage();

    let damage = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == priest)
        .map_or(u16::MAX, |permanent| permanent.damage);
    assert_eq!(damage, 0, "a 0/8 Wall deals it nothing");
}

/// Lure puts the same requirement on whatever it enchants, and names every
/// creature rather than only Walls.
#[test]
fn lure_pulls_every_creature_onto_its_host() {
    let (mut game, [host, _]) = two_attackers(cards::GRIZZLY_BEARS, cards::SEDGE_TROLL);
    let lion = add_blocker(&mut game, 10_010, cards::SAVANNAH_LIONS);
    assert_eq!(seats(&game, lion).len(), 2, "free before the Aura arrives");

    let mut lure = creature(10_020, cards::LURE, PlayerId::One);
    lure.attached_to = Some(host);
    game.battlefield.push(lure);

    assert_eq!(seats(&game, lion), vec![host]);
    assert!(!may_finish(&game));
}

/// A ground creature is not able to block a flier, so the Aura's requirement
/// leaves it alone -- the restriction wins.
#[test]
fn a_restriction_beats_the_requirement() {
    let (mut game, [host, other]) = two_attackers(cards::SERRA_ANGEL, cards::SEDGE_TROLL);
    let lion = add_blocker(&mut game, 10_010, cards::SAVANNAH_LIONS);
    let mut lure = creature(10_020, cards::LURE, PlayerId::One);
    lure.attached_to = Some(host);
    game.battlefield.push(lure);

    assert_eq!(
        seats(&game, lion),
        vec![other],
        "it cannot block the flier, so it keeps the seat it can take"
    );
    assert!(may_finish(&game));
}

#[test]
fn every_must_block_identity_reports_complete_coverage() {
    let catalog = poc::catalog().expect("catalog builds");
    for definition in [cards::LURE, cards::MARBLE_PRIEST] {
        let card = catalog.get(definition).expect("the card is cataloged");
        assert_eq!(
            card.rules.implementation_status(),
            ImplementationStatus::Complete,
            "{} should be fully executable",
            card.name,
        );
    }
}
