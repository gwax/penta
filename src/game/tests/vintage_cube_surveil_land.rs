//! Thundering Falls: a tapped dual that looks at the top of your library on
//! the way in and lets you bin what you find.

use super::*;

/// Player One with a Thundering Falls in hand and a known card on top.
fn staged(top: CardDefinitionId) -> (Game, GameObjectId) {
    staged_with(cards::THUNDERING_FALLS, top)
}

/// The same, for any land in the cycle.
fn staged_with(land: CardDefinitionId, top: CardDefinitionId) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].library.clear();
    let card = game
        .build_zone(PlayerId::One, &[top])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    game.players[0].library.push(card);
    let land = game
        .build_zone(PlayerId::One, &[land])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let land_id = land.id;
    game.players[0].hand.push(land);
    (game, land_id)
}

/// Plays the land and answers the surveil, keeping the card on top when
/// `bin` is false.
fn play_and_surveil(game: &mut Game, land: GameObjectId, bin: bool) {
    game.priority = PlayerId::One;
    let play = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::PlayLand { card, .. } if *card == land))
        .expect("a land drop is available");
    game.apply(PlayerId::One, play)
        .expect("the land is playable");

    for _ in 0..12 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            // Choosing the card puts it in the graveyard; choosing nothing
            // leaves it where it was.
            let options = if bin {
                decision
                    .options
                    .first()
                    .map(|option| vec![option.id])
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("surveil accepts either answer");
            continue;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            return;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            return;
        }
    }
}

fn the_land(game: &Game) -> Option<&Permanent> {
    the_land_named(game, cards::THUNDERING_FALLS)
}

fn the_land_named(game: &Game, definition: CardDefinitionId) -> Option<&Permanent> {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == definition)
}

/// Every colour of mana this permanent will make.
fn colors_of(game: &Game, id: GameObjectId) -> Vec<ManaColor> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::ActivateManaAbility { source, color, .. } if source == id => Some(color),
            _ => None,
        })
        .collect()
}

/// The check every member of the cycle gets: play it, watch it arrive
/// tapped, bin what the surveil turned up, and confirm the two colours its
/// basic types make. The cycle is one card printed six ways, so what is
/// worth asserting per member is which two.
fn cycle_member_makes(land: CardDefinitionId, first: ManaColor, second: ManaColor) {
    let (mut game, card) = staged_with(land, cards::LIGHTNING_BOLT);
    play_and_surveil(&mut game, card, true);
    let permanent = the_land_named(&game, land).expect("it is on the battlefield");
    let id = permanent.card.id;
    assert!(permanent.tapped, "tapped on arrival");
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::LIGHTNING_BOLT),
        "and its surveil bins what it was told to",
    );
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == id)
    {
        permanent.tapped = false;
    }

    let colors = colors_of(&game, id);
    assert!(colors.contains(&first));
    assert!(colors.contains(&second));
    assert_eq!(colors.len(), 2, "and nothing else");
}

/// It arrives tapped, whatever you do with the surveil.
#[test]
fn it_enters_tapped() {
    let (mut game, land) = staged(cards::LIGHTNING_BOLT);
    play_and_surveil(&mut game, land, false);

    assert!(the_land(&game).is_some_and(|permanent| permanent.tapped));
}

/// Binning the card puts it in the graveyard and empties the library.
#[test]
fn surveil_may_put_the_card_in_the_graveyard() {
    let (mut game, land) = staged(cards::LIGHTNING_BOLT);
    play_and_surveil(&mut game, land, true);

    assert!(game.players[0].library.is_empty(), "it left the top");
    assert_eq!(
        game.players[0]
            .graveyard
            .iter()
            .filter(|card| card.definition == cards::LIGHTNING_BOLT)
            .count(),
        1,
    );
}

/// Declining leaves it on top, which is the other half of the choice.
#[test]
fn surveil_may_leave_the_card_on_top() {
    let (mut game, land) = staged(cards::LIGHTNING_BOLT);
    play_and_surveil(&mut game, land, false);

    assert_eq!(game.players[0].library.len(), 1, "still on top");
    assert!(
        game.players[0].graveyard.is_empty(),
        "and nothing was binned",
    );
}

/// The mana abilities come from the basic land types rather than a printed
/// clause, so both colours are on offer.
#[test]
fn it_taps_for_either_colour() {
    let (mut game, land) = staged(cards::LIGHTNING_BOLT);
    play_and_surveil(&mut game, land, false);
    let id = the_land(&game).expect("it is on the battlefield").card.id;
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == id)
    {
        permanent.tapped = false;
    }

    let colors = colors_of(&game, id);
    assert!(colors.contains(&ManaColor::Blue), "Island");
    assert!(colors.contains(&ManaColor::Red), "Mountain");
}

/// The basic types are on the land itself rather than granted, which is
/// what makes the mana abilities appear at all.
#[test]
fn the_sewers_bring_their_own_two_basic_types() {
    let (mut game, land) = staged_with(cards::UNDERCITY_SEWERS, cards::MOUNTAIN);
    play_and_surveil(&mut game, land, false);

    let sewers = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::UNDERCITY_SEWERS)
        .expect("it is on the battlefield");
    let subtypes = game.effective_subtypes(sewers);

    assert!(subtypes.contains(&"Island"));
    assert!(subtypes.contains(&"Swamp"));
    assert!(sewers.tapped, "and it still arrives tapped");
}

/// The rest of the cycle, one line each: the same land with a different
/// pair of basic types on it.
#[test]
fn the_white_blue_land_taps_for_its_own_two() {
    cycle_member_makes(cards::METICULOUS_ARCHIVE, ManaColor::White, ManaColor::Blue);
}

#[test]
fn the_white_black_land_taps_for_its_own_two() {
    cycle_member_makes(
        cards::SHADOWY_BACKSTREET,
        ManaColor::White,
        ManaColor::Black,
    );
}

#[test]
fn the_blue_black_land_taps_for_its_own_two() {
    cycle_member_makes(cards::UNDERCITY_SEWERS, ManaColor::Blue, ManaColor::Black);
}

#[test]
fn the_black_red_land_taps_for_its_own_two() {
    cycle_member_makes(cards::RAUCOUS_THEATER, ManaColor::Black, ManaColor::Red);
}

#[test]
fn the_red_green_land_taps_for_its_own_two() {
    cycle_member_makes(cards::COMMERCIAL_DISTRICT, ManaColor::Red, ManaColor::Green);
}

#[test]
fn the_green_white_land_taps_for_its_own_two() {
    cycle_member_makes(cards::LUSH_PORTICO, ManaColor::Green, ManaColor::White);
}

#[test]
fn the_green_blue_land_taps_for_its_own_two() {
    cycle_member_makes(cards::HEDGE_MAZE, ManaColor::Green, ManaColor::Blue);
}

#[test]
fn the_black_green_land_taps_for_its_own_two() {
    cycle_member_makes(
        cards::UNDERGROUND_MORTUARY,
        ManaColor::Black,
        ManaColor::Green,
    );
}
