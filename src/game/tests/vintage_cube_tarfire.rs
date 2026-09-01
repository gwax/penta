//! Tarfire: a Shock that is also a Goblin card, which is the only reason it
//! is in the deck.

use super::*;

/// Player One holding a Tarfire with one red up.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let tarfire = game
        .build_zone(PlayerId::One, &[cards::TARFIRE])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = tarfire.id;
    game.players[0].hand.push(tarfire);
    game.turns_started = [2, 1];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    (game, id)
}

fn cast_at(game: &mut Game, tarfire: GameObjectId, target: Target) {
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == tarfire && choices.iter_targets().any(|chosen| *chosen == target)
            }
            _ => false,
        })
        .expect("the target is legal");
    game.apply(PlayerId::One, cast).expect("it is cast");
    for _ in 0..8 {
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

/// Two damage, at a face or at a creature.
#[test]
fn it_deals_two_damage_to_any_target() {
    let (mut game, tarfire) = staged();

    cast_at(&mut game, tarfire, Target::Player(PlayerId::Two));
    assert_eq!(game.players[1].life, 18);

    let (mut game, tarfire) = staged();
    let bears = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);

    cast_at(&mut game, tarfire, Target::Permanent(bears));

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == bears),
        "two kills a 2/2",
    );
}

/// The type line is the card: an instant that is also Kindred, and a Goblin.
#[test]
fn it_is_a_kindred_instant_goblin() {
    let catalog = crate::poc::catalog().expect("catalog builds");
    let definition = catalog.get(cards::TARFIRE).expect("cataloged");

    assert!(definition.rules.has_type(CardType::Instant));
    assert!(
        definition.rules.has_type(CardType::Kindred),
        "kindred is what carries the subtype",
    );
    assert!(
        !definition.rules.has_type(CardType::Creature),
        "and it is no creature"
    );
    assert_eq!(definition.rules.subtypes(), ["Goblin"]);
}

/// The whole point: a Goblin Ringleader finds it in the library, because it
/// is a Goblin card there like anywhere else.
#[test]
fn a_ringleader_draws_it_as_a_goblin_card() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    // The library reads from the back, so these are the top four.
    for definition in [
        cards::ISLAND,
        cards::GOBLIN_LACKEY,
        cards::MOUNTAIN,
        cards::TARFIRE,
    ] {
        let card = game
            .build_zone(PlayerId::One, &[definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        game.players[0].library.push(card);
    }

    game.put_onto_battlefield(PlayerId::One, cards::GOBLIN_RINGLEADER)
        .expect("cataloged");
    drain_pending(&mut game);

    let hand = game.players[0]
        .hand
        .iter()
        .map(|card| card.definition)
        .collect::<Vec<_>>();
    assert!(
        hand.contains(&cards::TARFIRE),
        "the Ringleader took the Tarfire: {hand:?}",
    );
    assert!(hand.contains(&cards::GOBLIN_LACKEY), "and the Goblin");
    assert_eq!(hand.len(), 2, "and neither of the lands");
}

/// "Kindred is a card type and will be counted by effects that refer to the
/// number of card types among cards in a zone." A Tarfire in the graveyard
/// is two of them at once, which a Lhurgoyf reads as two.
#[test]
fn it_is_two_card_types_to_a_lhurgoyf() {
    let size_with = |definition: CardDefinitionId| {
        let mut game = ready_game();
        game.battlefield.clear();
        game.players[PlayerId::One.index()].graveyard.clear();
        game.players[PlayerId::Two.index()].graveyard.clear();
        game.players[PlayerId::One.index()]
            .graveyard
            .push(card(96_800, definition, PlayerId::One));
        let goyf = game
            .put_onto_battlefield(PlayerId::One, cards::PYROGOYF)
            .expect("cataloged");
        let permanent = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == goyf)
            .expect("it is there");
        (game.power(permanent), game.toughness(permanent))
    };

    assert_eq!(
        size_with(cards::LIGHTNING_BOLT),
        (Some(1), Some(2)),
        "an ordinary instant is one card type",
    );
    assert_eq!(
        size_with(cards::TARFIRE),
        (Some(2), Some(3)),
        "and a Kindred Instant is two, off the one card",
    );
}
