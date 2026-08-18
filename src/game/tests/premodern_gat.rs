//! The cards Daniel Sondike's GAT list needed.
//!
//! Each of these turns on a piece of engine vocabulary the format had not
//! asked for before: a scalar the source chose on entry and then reads back,
//! a land that spends itself, and a replacement paid from hand.

use super::*;

/// Casts `definition` from `player`'s hand with the mana already available,
/// letting the spell resolve.
fn cast_permanent(game: &mut Game, id: u32, definition: CardDefinitionId, player: PlayerId) {
    let spell = card(id, definition, player);
    let spell_id = spell.id;
    game.players[player.index()].hand.push(spell);
    let pool = &mut game.players[player.index()].mana_pool;
    pool.black = 3;
    pool.colorless = 3;
    game.priority = player;
    game.apply(player, cast_action(spell_id, Vec::new(), Vec::new(), 0))
        .expect("the spell is cast");
    pass_priority_pair(game);
}

/// Answers the as-enters scalar choice with `label`.
fn choose_scalar(game: &mut Game, player: PlayerId, label: &str) {
    let decision = game
        .observe(player)
        .decision
        .expect("the permanent asks for its entry choice");
    let option = decision
        .options
        .iter()
        .find(|option| option.label == label)
        .unwrap_or_else(|| panic!("{label} is offered"))
        .id;
    game.apply(
        player,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![option],
        },
    )
    .expect("the choice is legal");
    drain_pending(game);
}

fn stats(game: &Game, id: GameObjectId) -> Option<(i16, i16)> {
    let permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)?;
    Some((
        game.power(permanent).expect("a creature has power"),
        game.toughness(permanent).expect("a creature has toughness"),
    ))
}

#[test]
fn engineered_plague_shrinks_only_the_type_it_named() {
    let mut game = ready_game();
    let ringleader = creature(10_001, cards::GOBLIN_RINGLEADER, PlayerId::Two);
    let ringleader_id = ringleader.card.id;
    game.battlefield.push(ringleader);
    let dryad = creature(10_002, cards::QUIRION_DRYAD, PlayerId::Two);
    let dryad_id = dryad.card.id;
    game.battlefield.push(dryad);

    cast_permanent(&mut game, 10_000, cards::ENGINEERED_PLAGUE, PlayerId::One);
    choose_scalar(&mut game, PlayerId::One, "Goblin");

    assert_eq!(
        stats(&game, ringleader_id),
        Some((1, 1)),
        "the Goblin shrank"
    );
    assert_eq!(
        stats(&game, dryad_id),
        Some((1, 1)),
        "the Dryad is untouched, and its own 1/1 is not the Plague's doing",
    );
}

/// The half that decides games: a one-toughness creature of the named type is
/// not shrunk but killed, by state-based actions rather than by the Plague.
#[test]
fn engineered_plague_kills_the_one_toughness_creatures_it_names() {
    let mut game = ready_game();
    let fanatic = creature(10_001, cards::MOGG_FANATIC, PlayerId::Two);
    let fanatic_id = fanatic.card.id;
    game.battlefield.push(fanatic);

    cast_permanent(&mut game, 10_000, cards::ENGINEERED_PLAGUE, PlayerId::One);
    choose_scalar(&mut game, PlayerId::One, "Goblin");

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == fanatic_id),
        "a 1/1 Goblin is a 0/0 and dies",
    );
}

/// A Plague that never got its choice -- and one that named something else --
/// leaves everything alone. The predicate reads a specific chosen type, not
/// "some type was chosen".
#[test]
fn a_plague_naming_another_type_shrinks_nothing() {
    let mut game = ready_game();
    let ringleader = creature(10_001, cards::GOBLIN_RINGLEADER, PlayerId::Two);
    let ringleader_id = ringleader.card.id;
    game.battlefield.push(ringleader);

    cast_permanent(&mut game, 10_000, cards::ENGINEERED_PLAGUE, PlayerId::One);
    choose_scalar(&mut game, PlayerId::One, "Dryad");

    assert_eq!(stats(&game, ringleader_id), Some((2, 2)));
}

/// Plays `definition` as a land from `player`'s hand and returns its id.
fn play_land(
    game: &mut Game,
    id: u32,
    definition: CardDefinitionId,
    player: PlayerId,
) -> GameObjectId {
    let land = card(id, definition, player);
    let land_id = land.id;
    game.players[player.index()].hand.push(land);
    game.priority = player;
    game.apply(
        player,
        Action::PlayLand {
            card: land_id,
            option: PlayOptionId::DEFAULT,
        },
    )
    .expect("the land is played");
    drain_pending(game);
    // A permanent entering the battlefield is a new object, so the card's id
    // in hand is not the one to follow afterwards.
    let _ = land_id;
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == definition)
        .expect("the land entered the battlefield")
        .card
        .id
}

fn mining_counters(game: &Game, id: GameObjectId) -> Option<u16> {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .map(|permanent| permanent.counters(CounterKind::Mining))
}

/// Taps the Mine for one mana of `color`, then untaps it so the next
/// activation is about the counters rather than about the tap.
fn mine_for(game: &mut Game, source: GameObjectId, color: ManaColor) {
    game.apply(
        PlayerId::One,
        Action::ActivateManaAbility {
            source,
            ability: mana_ability_for(game, source, color),
            color,
            counters_removed: None,
            cost_object: None,
        },
    )
    .expect("the Mine still has a counter to spend");
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == source)
    {
        permanent.tapped = false;
    }
}

#[test]
fn gemstone_mine_enters_with_three_counters_and_spends_itself_dry() {
    let mut game = ready_game();
    let mine = play_land(&mut game, 10_000, cards::GEMSTONE_MINE, PlayerId::One);
    assert_eq!(mining_counters(&game, mine), Some(3));

    // Every colour, because that is the whole reason the deck plays it.
    mine_for(&mut game, mine, ManaColor::Blue);
    assert_eq!(mining_counters(&game, mine), Some(2));
    mine_for(&mut game, mine, ManaColor::White);
    assert_eq!(mining_counters(&game, mine), Some(1));
    assert_eq!(game.players[PlayerId::One.index()].mana_pool.blue, 1);
    assert_eq!(game.players[PlayerId::One.index()].mana_pool.white, 1);

    mine_for(&mut game, mine, ManaColor::Green);
    assert_eq!(
        mining_counters(&game, mine),
        None,
        "the last counter takes the land with it",
    );
    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::GEMSTONE_MINE),
        "and it goes to the graveyard, not into exile or nowhere",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].mana_pool.green,
        1,
        "the mana it was sacrificed for is still in the pool",
    );
}

/// The Mine is offered only while it can pay its own cost.
#[test]
fn a_spent_mine_offers_nothing() {
    let mut game = ready_game();
    let mine = play_land(&mut game, 10_000, cards::GEMSTONE_MINE, PlayerId::One);
    for color in [ManaColor::Blue, ManaColor::White, ManaColor::Green] {
        mine_for(&mut game, mine, color);
    }

    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(
                action,
                Action::ActivateManaAbility { source, .. } if *source == mine
            )),
        "a land in the graveyard is not a mana source",
    );
}
