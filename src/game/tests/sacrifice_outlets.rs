//! Activated abilities whose cost removes the permanent that is activating
//! them. The source is already in the graveyard when the effect resolves, so
//! what has to hold is that the effect still lands -- and, for the outlet
//! that eats somebody else, that the cost takes the creature it named rather
//! than the one activating.

use super::*;

fn ready() -> Game {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    game.turns_started[PlayerId::One.index()] = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game
}

fn resolve(game: &mut Game) {
    for _ in 0..12 {
        drain_pending(game);
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let holder = game.priority;
        if game.apply(holder, Action::PassPriority).is_err() {
            break;
        }
    }
}

fn count(game: &Game, definition: CardDefinitionId) -> usize {
    game.battlefield
        .iter()
        .filter(|permanent| permanent.card.definition == ObjectKind::Card(definition))
        .count()
}

/// Activates the one ability whose target is `target`, or the only one there
/// is when `target` is `None`.
fn activate(game: &mut Game, source: GameObjectId, target: Option<GameObjectId>) {
    let chosen = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source: actual,
                targets,
                ..
            } => {
                *actual == source
                    && match target {
                        Some(wanted) => targets.first().is_some_and(|selection| {
                            selection.targets() == [Target::Permanent(wanted)]
                        }),
                        None => true,
                    }
            }
            _ => false,
        })
        .expect("the ability is offered");
    game.apply(PlayerId::One, chosen).expect("it activates");
    resolve(game);
}

/// The Debaser is in the graveyard before its own ability resolves, and the
/// -2/-2 still has to land on what it named.
#[test]
fn the_debaser_shrinks_its_target_from_the_graveyard() {
    let mut game = ready();
    let mut debaser = creature(81_000, cards::PHYREXIAN_DEBASER, PlayerId::One);
    debaser.entered_controller_turn = 0;
    let debaser_id = debaser.card.id;
    game.battlefield.push(debaser);
    let bears = creature(81_001, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);

    activate(&mut game, debaser_id, Some(bears_id));

    assert_eq!(
        count(&game, cards::PHYREXIAN_DEBASER),
        0,
        "it paid with itself"
    );
    assert_eq!(
        count(&game, cards::GRIZZLY_BEARS),
        0,
        "and a 2/2 given -2/-2 dies with it"
    );
}

/// The Plaguelord's second outlet eats a different creature, so the cost has
/// to take the one it named and leave the Plaguelord standing to do it again.
#[test]
fn the_plaguelord_eats_somebody_else() {
    let mut game = ready();
    let mut lord = creature(81_100, cards::PHYREXIAN_PLAGUELORD, PlayerId::One);
    lord.entered_controller_turn = 0;
    let lord_id = lord.card.id;
    game.battlefield.push(lord);
    let lions = creature(81_101, cards::SAVANNAH_LIONS, PlayerId::One);
    let lions_id = lions.card.id;
    game.battlefield.push(lions);
    let victim = creature(81_102, cards::RAGING_GOBLIN, PlayerId::Two);
    let victim_id = victim.card.id;
    game.battlefield.push(victim);

    let feeding = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source,
                cost_objects,
                targets,
                ..
            } => {
                *source == lord_id
                    && cost_objects == &[lions_id]
                    && targets.first().is_some_and(|selection| {
                        selection.targets() == [Target::Permanent(victim_id)]
                    })
            }
            _ => false,
        })
        .expect("feeding the Lions to the Plaguelord is offered");
    game.apply(PlayerId::One, feeding).expect("it activates");
    resolve(&mut game);

    assert_eq!(
        count(&game, cards::PHYREXIAN_PLAGUELORD),
        1,
        "the Plaguelord is not what it ate"
    );
    assert_eq!(
        count(&game, cards::SAVANNAH_LIONS),
        0,
        "the Lions paid the cost"
    );
    assert_eq!(
        count(&game, cards::RAGING_GOBLIN),
        0,
        "and the -1/-1 it bought killed a 1/1"
    );
}

/// The Necromancer's target is a card in a graveyard rather than a permanent,
/// and what comes back has to arrive on the battlefield.
#[test]
fn the_doomed_necromancer_brings_a_creature_back() {
    let mut game = ready();
    let mut necromancer = creature(81_200, cards::DOOMED_NECROMANCER, PlayerId::One);
    necromancer.entered_controller_turn = 0;
    let necromancer_id = necromancer.card.id;
    game.battlefield.push(necromancer);
    game.players[0]
        .graveyard
        .push(card(81_201, cards::SERRA_ANGEL, PlayerId::One));
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);

    assert_eq!(
        count(&game, cards::SERRA_ANGEL),
        0,
        "still in the graveyard"
    );
    activate(&mut game, necromancer_id, None);

    assert_eq!(
        count(&game, cards::DOOMED_NECROMANCER),
        0,
        "the Necromancer spent itself"
    );
    assert_eq!(
        count(&game, cards::SERRA_ANGEL),
        1,
        "and the Angel is on the battlefield"
    );
    assert!(
        !game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == ObjectKind::Card(cards::SERRA_ANGEL)),
        "not left behind in the graveyard as well"
    );
}
