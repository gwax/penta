//! Knight of the Reliquary: a body that grows a point every time it fetches,
//! which is what makes the utility lands in the deck worth a card each.

use super::*;

/// The Knight on the battlefield since last turn, with `lands` beside her,
/// `graveyard` behind her, and `library` to search.
fn staged(
    lands: &[CardDefinitionId],
    graveyard: &[CardDefinitionId],
    library: &[CardDefinitionId],
) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    game.players[0].library.clear();
    for (index, definition) in graveyard.iter().enumerate() {
        game.players[0].graveyard.push(card(
            91_000 + u32::try_from(index).expect("a small graveyard"),
            *definition,
            PlayerId::One,
        ));
    }
    for (index, definition) in library.iter().enumerate() {
        game.players[0].library.push(card(
            91_500 + u32::try_from(index).expect("a small library"),
            *definition,
            PlayerId::One,
        ));
    }
    let knight = game
        .put_onto_battlefield(PlayerId::One, cards::KNIGHT_OF_THE_RELIQUARY)
        .expect("cataloged");
    let mut ids = Vec::new();
    for definition in lands {
        ids.push(
            game.put_onto_battlefield(PlayerId::One, *definition)
                .expect("cataloged"),
        );
    }
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [2, 1];
    drain_pending(&mut game);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, knight, ids)
}

fn settle(game: &mut Game) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .take(decision.minimum.max(1))
                .map(|option| option.id)
                .collect();
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the offered choice is legal");
            continue;
        }
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

fn permanent(game: &Game, id: GameObjectId) -> &Permanent {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("it is on the battlefield")
}

/// Every fetch she is offering, by the land it would eat.
fn fetches(game: &Game, knight: GameObjectId) -> Vec<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == knight),
        )
        .collect()
}

/// She counts land cards in your graveyard, and nothing else in it.
#[test]
fn she_grows_with_the_lands_in_your_graveyard() {
    let (game, knight, _) = staged(
        &[],
        &[cards::FOREST, cards::WASTELAND, cards::SERRA_ANGEL],
        &[],
    );

    let knight = permanent(&game, knight);
    assert_eq!(game.power(knight), Some(4), "two lands, not the Angel");
    assert_eq!(game.toughness(knight), Some(4));
}

/// Eating a Plains fetches any land at all, untapped, and the Plains it ate
/// makes her bigger on the way past.
#[test]
fn sacrificing_a_plains_fetches_an_untapped_land() {
    let (mut game, knight, ids) = staged(&[cards::PLAINS], &[], &[cards::WASTELAND]);

    let action = fetches(&game, knight)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility { cost_objects, .. } => cost_objects.contains(&ids[0]),
            _ => false,
        })
        .expect("the Plains can be sacrificed");
    game.apply(PlayerId::One, action).expect("it activates");
    settle(&mut game);

    let fetched = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::WASTELAND)
        .expect("the Wasteland was found");
    assert!(!fetched.tapped, "and it arrives untapped");
    let knight = permanent(&game, knight);
    assert!(knight.tapped, "she tapped to do it");
    assert_eq!(
        game.power(knight),
        Some(3),
        "and the Plains she ate now counts from the graveyard",
    );
}

/// "A Forest or Plains" is read off the basic land types, so a dual with
/// either one pays for her.
#[test]
fn a_dual_with_either_type_pays_for_it() {
    let (game, knight, ids) = staged(&[cards::TAIGA], &[], &[cards::WASTELAND]);

    assert!(
        fetches(&game, knight).iter().any(|action| match action {
            Action::ActivateAbility { cost_objects, .. } => cost_objects.contains(&ids[0]),
            _ => false,
        }),
        "a Mountain Forest is a Forest",
    );
}

/// A land that is neither cannot pay.
#[test]
fn another_land_cannot_pay() {
    let (game, knight, _) = staged(&[cards::SWAMP], &[], &[cards::WASTELAND]);

    assert!(
        fetches(&game, knight).is_empty(),
        "a Swamp is not a Forest or a Plains",
    );
}

/// Her ruling: the bonus applies only on the battlefield, and in every other
/// zone she is the 2/2 she prints. Corpse Lunge reads the power of the card
/// it exiled out of the graveyard, and what it reads is two -- three lands
/// buried beside her included.
#[test]
fn she_is_a_two_two_outside_the_battlefield() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    for (index, definition) in [
        cards::MOUNTAIN,
        cards::FOREST,
        cards::PLAINS,
        cards::KNIGHT_OF_THE_RELIQUARY,
    ]
    .into_iter()
    .enumerate()
    {
        game.players[0].graveyard.push(card(
            91_900 + u32::try_from(index).expect("a small graveyard"),
            definition,
            PlayerId::One,
        ));
    }
    let wall = game
        .put_onto_battlefield(PlayerId::Two, cards::LIVING_WALL)
        .expect("cataloged");
    drain_pending(&mut game);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    let lunge = game
        .build_zone(PlayerId::One, &[cards::CORPSE_LUNGE])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let lunge_id = lunge.id;
    game.players[0].hand.push(lunge);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 3);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == lunge_id))
        .expect("the Knight is the creature card it exiles");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game);

    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == wall)
            .expect("a 0/6 survives it")
            .damage,
        2,
        "her printed two, with the three lands counting for nothing",
    );
}

/// "Each land card in your graveyard": the yard across the table is no part
/// of it, however many fetches they have cracked. A Knight with one land
/// behind her is a 3/3 beside an opponent's four.
#[test]
fn their_graveyard_does_not_grow_her() {
    let (mut game, knight, _) = staged(&[], &[cards::FOREST], &[]);
    for (index, definition) in [
        cards::PLAINS,
        cards::WASTELAND,
        cards::TAIGA,
        cards::VOLCANIC_ISLAND,
    ]
    .into_iter()
    .enumerate()
    {
        game.players[1].graveyard.push(card(
            92_000 + u32::try_from(index).expect("a small graveyard"),
            definition,
            PlayerId::Two,
        ));
    }

    let she = permanent(&game, knight);
    assert_eq!(
        game.power(she),
        Some(3),
        "her own one land, and not their four",
    );
    assert_eq!(game.toughness(she), Some(3));

    // And the count is live: a land of yours reaching the graveyard is worth
    // a counter's worth of size the moment it lands there.
    game.players[0]
        .graveyard
        .push(card(92_100, cards::PLAINS, PlayerId::One));
    let she = permanent(&game, knight);
    assert_eq!(game.power(she), Some(4), "the second of yours does count");
}
