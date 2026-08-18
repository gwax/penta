//! The cards the Premodern Landstill list needed.

use super::*;

fn put_in_graveyard(game: &mut Game, id: u32, definition: CardDefinitionId, owner: PlayerId) {
    let card = card(id, definition, owner);
    game.players[owner.index()].graveyard.push(card);
}

fn graveyard_definitions(game: &Game, player: PlayerId) -> Vec<CardDefinitionId> {
    game.players[player.index()]
        .graveyard
        .iter()
        .map(|card| card.definition)
        .collect()
}

#[test]
fn phyrexian_furnace_eats_a_graveyard_from_the_bottom() {
    let mut game = ready_game();
    let furnace = creature(10_000, cards::PHYREXIAN_FURNACE, PlayerId::One);
    let furnace_id = furnace.card.id;
    game.battlefield.push(furnace);
    // Oldest first, which is the bottom of the pile.
    put_in_graveyard(&mut game, 10_001, cards::LIGHTNING_BOLT, PlayerId::Two);
    put_in_graveyard(&mut game, 10_002, cards::COUNTERSPELL, PlayerId::Two);

    let activation = Action::ActivateAbility {
        source: furnace_id,
        ability: primary_ability(cards::PHYREXIAN_FURNACE),
        targets: activated_targets(Target::Player(PlayerId::Two)),
        cost_objects: Vec::new(),
        x: 0,
    };
    assert!(game.legal_actions(PlayerId::One).contains(&activation));
    game.apply(PlayerId::One, activation).unwrap();
    pass_priority_pair(&mut game);

    assert_eq!(
        graveyard_definitions(&game, PlayerId::Two),
        vec![cards::COUNTERSPELL],
        "the oldest card went, not the newest",
    );
    assert!(
        game.players[PlayerId::Two.index()]
            .exile
            .iter()
            .any(|card| card.definition == cards::LIGHTNING_BOLT),
        "and it is in exile",
    );
}

/// The other mode picks the card that mattered, and pays for itself.
#[test]
fn sacrificing_the_furnace_exiles_a_chosen_card_and_draws() {
    let mut game = ready_game();
    let furnace = creature(10_000, cards::PHYREXIAN_FURNACE, PlayerId::One);
    let furnace_id = furnace.card.id;
    game.battlefield.push(furnace);
    put_in_graveyard(&mut game, 10_001, cards::LIGHTNING_BOLT, PlayerId::Two);
    put_in_graveyard(&mut game, 10_002, cards::COUNTERSPELL, PlayerId::Two);
    game.players[PlayerId::One.index()].mana_pool.colorless = 1;
    let before = game.players[PlayerId::One.index()].hand.len();

    let newest = game.players[PlayerId::Two.index()]
        .graveyard
        .last()
        .expect("the graveyard has cards")
        .id;
    let activation = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                Action::ActivateAbility { source, targets, .. }
                    if *source == furnace_id
                        && targets.iter().any(|selection| {
                            selection.targets().contains(&Target::Card(newest))
                        })
            )
        })
        .expect("the newest card is targetable too");
    game.apply(PlayerId::One, activation).unwrap();
    pass_priority_pair(&mut game);

    assert_eq!(
        graveyard_definitions(&game, PlayerId::Two),
        vec![cards::LIGHTNING_BOLT],
        "the chosen card went rather than the bottom one",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].hand.len(),
        before + 1,
        "and the Furnace replaced itself",
    );
}

#[test]
fn powder_keg_destroys_what_its_fuse_counters_name() {
    let mut game = ready_game();
    let keg = creature(10_000, cards::POWDER_KEG, PlayerId::One);
    let keg_id = keg.card.id;
    game.battlefield.push(keg);
    // Two fuse counters, so two-drops die and nothing else does.
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == keg_id)
    {
        permanent.add_counters(CounterKind::Fuse, 2);
    }
    let two_drop = creature(10_001, cards::QUIRION_DRYAD, PlayerId::Two);
    let two_drop_id = two_drop.card.id;
    game.battlefield.push(two_drop);
    let one_drop = creature(10_002, cards::MOGG_FANATIC, PlayerId::Two);
    let one_drop_id = one_drop.card.id;
    game.battlefield.push(one_drop);

    let activation = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == keg_id),
        )
        .expect("the Keg can be detonated");
    game.apply(PlayerId::One, activation).unwrap();
    pass_priority_pair(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == two_drop_id),
        "a two-drop matched two fuse counters",
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == one_drop_id),
        "a one-drop did not",
    );
}
