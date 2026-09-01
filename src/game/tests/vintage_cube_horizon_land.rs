//! The horizon lands: two colours that cost a life apiece, and a land that
//! turns into a card once it has nothing left to pay for. Sunbaked Canyon
//! stands for the cycle; the others differ only in which two colours.

use super::*;

fn staged() -> (Game, GameObjectId) {
    staged_land(cards::SUNBAKED_CANYON)
}

/// One horizon land of the cycle, alone on the battlefield.
fn staged_land(definition: CardDefinitionId) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    let land = game
        .put_onto_battlefield(PlayerId::One, definition)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;
    (game, land)
}

/// Cashes `land` in for a card, which every member of the cycle does the
/// same way. Taken from what the land is offering rather than by index: the
/// mana ability and this one are not numbered in the same sequence.
fn cash_in(game: &mut Game, land: GameObjectId) {
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == land))
        .expect("one mana and a tap buys a card");
    game.apply(PlayerId::One, action)
        .expect("the ability activates");
    drain_pending(game);
}

fn mana_colors(game: &Game, source: GameObjectId) -> Vec<ManaColor> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::ActivateManaAbility {
                source: id, color, ..
            } if id == source => Some(color),
            _ => None,
        })
        .collect()
}

/// Red and white, and nothing else -- a horizon land makes no colourless.
#[test]
fn it_offers_only_its_two_colours() {
    let (game, canyon) = staged();
    let colors = mana_colors(&game, canyon);

    assert!(colors.contains(&ManaColor::Red));
    assert!(colors.contains(&ManaColor::White));
    assert_eq!(colors.len(), 2, "no colourless and no third colour");
}

/// Tapping it costs a life, which is the whole reason it draws a card later.
#[test]
fn making_mana_costs_a_life() {
    let (mut game, canyon) = staged();
    let ability = mana_ability_for(&game, canyon, ManaColor::Red);

    game.apply(
        PlayerId::One,
        Action::ActivateManaAbility {
            source: canyon,
            ability,
            color: ManaColor::Red,
            counters_removed: None,
            cost_object: None,
            combination: None,
            triggered_mana: None,
        },
    )
    .expect("a land with a life to spare taps for red");

    assert_eq!(game.players[0].mana_pool.red, 1);
    assert_eq!(game.players[0].life, 19, "one life for one mana");
}

/// A player at one life may still pay it: life is payable down to zero.
#[test]
fn one_life_is_still_enough() {
    let (mut game, canyon) = staged();
    game.players[0].life = 1;

    assert!(
        !mana_colors(&game, canyon).is_empty(),
        "paying to zero is legal (CR 118.4)",
    );
}

/// A player at zero cannot pay it, so the land makes no mana at all.
#[test]
fn no_life_means_no_mana() {
    let (mut game, canyon) = staged();
    game.players[0].life = 0;

    assert!(
        mana_colors(&game, canyon).is_empty(),
        "there is no life left to spend",
    );
}

/// Cashing it in sacrifices the land and draws.
#[test]
fn it_can_be_cashed_in_for_a_card() {
    let (mut game, canyon) = staged();
    let before = game.players[0].hand.len();

    cash_in(&mut game, canyon);

    assert!(
        game.battlefield.is_empty(),
        "the land sacrificed itself as a cost",
    );
    assert_eq!(game.players[0].hand.len(), before + 1);
    assert_eq!(game.players[0].life, 20, "and cost no life to do it");
}

/// Horizon Canopy is the original of the cycle and the same card: only the
/// pair of colours is different.
#[test]
fn the_canopy_offers_its_own_two_colours() {
    let (game, canopy) = staged_land(cards::HORIZON_CANOPY);
    let colors = mana_colors(&game, canopy);

    assert!(colors.contains(&ManaColor::Green), "Forest half");
    assert!(colors.contains(&ManaColor::White), "Plains half");
    assert_eq!(colors.len(), 2, "no colourless and no third colour");
}

/// And it cashes itself in the same way, which is what makes the shared
/// clause shared rather than copied.
#[test]
fn the_canopy_cashes_itself_in_too() {
    let (mut game, canopy) = staged_land(cards::HORIZON_CANOPY);
    let before = game.players[0].hand.len();

    cash_in(&mut game, canopy);

    assert!(game.battlefield.is_empty(), "the land sacrificed itself");
    assert_eq!(game.players[0].hand.len(), before + 1);
    assert_eq!(game.players[0].life, 20, "and cost no life to do it");
}

/// Waterlogged Grove is the Simic member: green and blue on the same terms,
/// down to the life and the card at the end.
#[test]
fn the_grove_is_the_green_and_blue_one() {
    let (mut game, grove) = staged_land(cards::WATERLOGGED_GROVE);
    let colors = mana_colors(&game, grove);
    assert!(colors.contains(&ManaColor::Green), "Forest half");
    assert!(colors.contains(&ManaColor::Blue), "Island half");
    assert_eq!(colors.len(), 2, "no colourless and no third colour");

    let ability = mana_ability_for(&game, grove, ManaColor::Blue);
    game.apply(
        PlayerId::One,
        Action::ActivateManaAbility {
            source: grove,
            ability,
            color: ManaColor::Blue,
            counters_removed: None,
            cost_object: None,
            combination: None,
            triggered_mana: None,
        },
    )
    .expect("it taps for blue");

    assert_eq!(game.players[0].mana_pool.blue, 1);
    assert_eq!(game.players[0].life, 19, "one life for one mana");
}

/// Once it has been tapped for mana the card is no longer available: the
/// draw wants the same tap the mana already spent.
#[test]
fn the_grove_cannot_both_pay_and_draw() {
    let (mut game, grove) = staged_land(cards::WATERLOGGED_GROVE);
    let before = game.players[0].hand.len();

    cash_in(&mut game, grove);

    assert_eq!(game.players[0].hand.len(), before + 1, "the card came");
    assert_eq!(game.players[0].life, 20, "and no life was spent on it");
    assert!(
        mana_colors(&game, grove).is_empty(),
        "sacrificed for the card, it makes no mana at all",
    );
}

/// Life paid is not damage taken: a prevention shield that would have eaten
/// an Ancient Tomb's two does nothing here, and the player who pays their
/// last life loses with the mana still in their pool.
#[test]
fn the_life_is_paid_rather_than_dealt() {
    let (mut game, canyon) = staged();
    let angel = card(97_900, cards::GUARDIAN_ANGEL, PlayerId::One);
    let angel_id = angel.id;
    game.players[0].hand.push(angel);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);
    game.apply(
        PlayerId::One,
        cast_action(angel_id, vec![Target::Player(PlayerId::One)], Vec::new(), 2),
    )
    .expect("a shield of two is castable");
    drain_pending(&mut game);
    game.empty_mana_pools();
    game.players[0].life = 1;

    let ability = mana_ability_for(&game, canyon, ManaColor::White);
    game.apply(
        PlayerId::One,
        Action::ActivateManaAbility {
            source: canyon,
            ability,
            color: ManaColor::White,
            counters_removed: None,
            cost_object: None,
            combination: None,
            triggered_mana: None,
        },
    )
    .expect("one life is enough to pay one life");
    game.check_state_based_actions();

    assert_eq!(
        game.players[0].life, 0,
        "the shield answers damage, and this is a payment",
    );
    assert_eq!(
        game.players[0].mana_pool.white, 1,
        "the mana was made all the same",
    );
    assert!(game.result.is_some(), "and zero life is a loss");
}

/// The draw wants a mana as well as the tap and the land: with an empty pool
/// there is nothing on offer but the mana ability itself.
#[test]
fn the_draw_wants_a_mana_of_its_own() {
    let (game, grove) = staged_land(cards::WATERLOGGED_GROVE);

    let offered = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter(|action| match action {
            Action::ActivateAbility { source, .. } => *source == grove,
            _ => false,
        })
        .count();
    assert_eq!(
        offered, 0,
        "an empty pool cannot pay the one the draw costs",
    );
    assert!(
        !mana_colors(&game, grove).is_empty(),
        "though the land still taps for its colours",
    );
}

/// The draw costs a mana, a tap and the land itself, and no life at all: a
/// player at one life may still cash the Grove in.
#[test]
fn the_grove_may_be_cashed_in_at_one_life() {
    let (mut game, grove) = staged_land(cards::WATERLOGGED_GROVE);
    game.players[0].life = 1;
    let before = game.players[0].hand.len();

    cash_in(&mut game, grove);

    assert_eq!(game.players[0].hand.len(), before + 1, "the card is drawn");
    assert_eq!(
        game.players[0].life, 1,
        "and the life is only what the mana ability charges",
    );
    assert!(game.result.is_none(), "so nobody dies for a card");
}

/// A horizon land is a plain Land: it makes two colours and carries neither
/// of their basic types, so a fetchland that reads for a Forest or an Island
/// walks straight past a Waterlogged Grove.
#[test]
fn a_fetchland_cannot_find_the_grove() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].library.clear();
    game.players[PlayerId::One.index()].library.push(card(
        98_000,
        cards::WATERLOGGED_GROVE,
        PlayerId::One,
    ));
    let fetch = game
        .put_onto_battlefield(PlayerId::One, cards::MISTY_RAINFOREST)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    let crack = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == fetch))
        .expect("a life and a sacrifice");
    game.apply(PlayerId::One, crack).expect("it activates");
    pass_priority_pair(&mut game);
    drain_pending(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::WATERLOGGED_GROVE),
        "the Grove says Land and nothing else, so there was nothing to find",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].library.len(),
        1,
        "and it is still in the library",
    );
}

/// The land is sacrificed to pay for the card, which makes cashing it in an
/// answer to the Strip Mine pointed at it: the cost is paid before their
/// ability resolves, and what resolves has nothing left to destroy.
#[test]
fn cashing_it_in_answers_a_strip_mine() {
    let (mut game, grove) = staged_land(cards::WATERLOGGED_GROVE);
    let forest = game
        .put_onto_battlefield(PlayerId::One, cards::FOREST)
        .expect("cataloged");
    let mine = game
        .put_onto_battlefield(PlayerId::Two, cards::STRIP_MINE)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::Two;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::Two;
    let library = game.players[PlayerId::One.index()].library.len();

    let blow_up = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } => {
                *source == mine
                    && targets
                        .iter()
                        .flat_map(crate::casting::TargetSelection::targets)
                        .any(|target| *target == Target::Permanent(grove))
            }
            _ => false,
        })
        .expect("the Grove is a land like any other");
    game.apply(PlayerId::Two, blow_up).expect("it activates");

    game.priority = PlayerId::One;
    cash_in(&mut game, grove);
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert_eq!(
        game.players[PlayerId::One.index()].library.len(),
        library - 1,
        "the card was drawn before their ability resolved",
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == forest),
        "and the ability found nothing else to take with it",
    );
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == mine),
        "the Strip Mine paid for itself either way",
    );
}

/// The two abilities differ in more than their cost: the mana ability is a
/// mana ability, so it resolves where it stands and nobody may answer it,
/// while the draw is an ordinary activated ability that waits on the stack.
#[test]
fn the_mana_resolves_at_once_and_the_draw_waits_on_the_stack() {
    let (mut game, canopy) = staged_land(cards::HORIZON_CANOPY);
    game.apply(
        PlayerId::One,
        Action::ActivateManaAbility {
            source: canopy,
            ability: mana_ability_for(&game, canopy, ManaColor::Green),
            color: ManaColor::Green,
            counters_removed: None,
            cost_object: None,
            combination: None,
            triggered_mana: None,
        },
    )
    .expect("a land with a life to spare taps for green");

    assert!(
        game.stack.is_empty(),
        "a mana ability uses no stack, so there was never a window",
    );
    assert_eq!(game.players[0].mana_pool.green, 1, "the mana is already up");

    // The same land, cashed in: this one is announced and then waits.
    let (mut game, canopy) = staged_land(cards::HORIZON_CANOPY);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    let hand = game.players[0].hand.len();
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == canopy),
        )
        .expect("one mana and a tap buys a card");
    game.apply(PlayerId::One, action).expect("it activates");

    assert_eq!(game.stack.len(), 1, "the draw is on the stack");
    assert_eq!(
        game.players[0].hand.len(),
        hand,
        "and nothing has been drawn yet",
    );
    assert!(
        game.battlefield.is_empty(),
        "though the land it cost is already gone",
    );

    drain_pending(&mut game);
    assert_eq!(
        game.players[0].hand.len(),
        hand + 1,
        "the card arrives when the ability resolves",
    );
}

/// The Grove as it is actually played: it has no clause to tap it, and a
/// land has no summoning sickness, so the turn it comes down is a turn it
/// pays for something.
#[test]
fn a_grove_played_this_turn_taps_the_same_turn() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    let held = card(43_000, cards::WATERLOGGED_GROVE, PlayerId::One);
    let held_id = held.id;
    game.players[PlayerId::One.index()].hand.push(held);
    game.players[PlayerId::One.index()].lands_played_this_turn = 0;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    let play = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::PlayLand { card, .. } if *card == held_id))
        .expect("a land drop is available");
    game.apply(PlayerId::One, play).expect("it is playable");
    drain_pending(&mut game);

    let grove = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::WATERLOGGED_GROVE)
        .expect("it was played");
    assert!(!grove.tapped, "a horizon land enters untapped");
    let grove_id = grove.card.id;
    assert_eq!(
        mana_colors(&game, grove_id),
        vec![ManaColor::Green, ManaColor::Blue],
        "and offers both its colours at once",
    );

    let tap = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                Action::ActivateManaAbility { source, color, .. }
                    if *source == grove_id && *color == ManaColor::Green
            )
        })
        .expect("green is on offer");
    game.apply(PlayerId::One, tap).expect("it taps");
    drain_pending(&mut game);

    assert_eq!(game.players[0].life, 19, "the life is paid on the way");
    assert_eq!(
        game.players[0].mana_pool.green, 1,
        "and the green is in the pool",
    );
}

/// The same land drop cashed in instead: with a mana from somewhere else the
/// Grove is a card the turn it arrives, without ever having made mana or
/// having paid a life.
#[test]
fn a_grove_played_this_turn_may_be_cashed_in_at_once() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    let held = card(43_100, cards::WATERLOGGED_GROVE, PlayerId::One);
    let held_id = held.id;
    game.players[PlayerId::One.index()].hand.push(held);
    game.players[PlayerId::One.index()].lands_played_this_turn = 0;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    let before = game.players[0].hand.len();

    let play = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::PlayLand { card, .. } if *card == held_id))
        .expect("a land drop is available");
    game.apply(PlayerId::One, play).expect("it is playable");
    drain_pending(&mut game);
    let grove = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::WATERLOGGED_GROVE)
        .expect("it was played")
        .card
        .id;

    cash_in(&mut game, grove);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == grove),
        "it sacrificed itself to pay",
    );
    assert_eq!(
        game.players[0].hand.len(),
        before,
        "the land left the hand and a card came back to it",
    );
    assert_eq!(game.players[0].life, 20, "and no life was paid for that");
}
