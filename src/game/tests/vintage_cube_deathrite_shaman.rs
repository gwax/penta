//! Deathrite Shaman: three abilities that eat a graveyard, and the first of
//! them makes mana without being a mana ability.

use super::*;

/// Her on the battlefield since last turn, with `mine` and `theirs` in the
/// two graveyards.
fn staged(mine: &[CardDefinitionId], theirs: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    game.players[1].graveyard.clear();
    for (player, definitions) in [(PlayerId::One, mine), (PlayerId::Two, theirs)] {
        for (index, definition) in definitions.iter().enumerate() {
            let id = 275_000
                + u32::from(player == PlayerId::Two) * 100
                + u32::try_from(index).expect("few cards");
            game.players[player.index()]
                .graveyard
                .push(card(id, *definition, player));
        }
    }
    let shaman = game
        .put_onto_battlefield(PlayerId::One, cards::DEATHRITE_SHAMAN)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, shaman)
}

/// Every way of activating her that taps for the land ability.
fn land_activations(game: &Game, shaman: GameObjectId) -> Vec<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(|action| {
            matches!(action, Action::ActivateAbility { source, ability, .. }
                if *source == shaman
                    && matches!(ability, AbilityOrigin::Printed { ability, .. } if *ability == AbilityId(0)))
        })
        .collect()
}

fn settle(game: &mut Game) {
    for _ in 0..16 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .map(|option| option.id)
                .take(decision.minimum.max(1))
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

/// It exiles the land and pays for it with a mana of whatever colour was
/// asked for.
#[test]
fn it_exiles_a_land_and_makes_a_mana() {
    let (mut game, shaman) = staged(&[cards::MOUNTAIN], &[]);

    let activation = land_activations(&game, shaman)
        .into_iter()
        .next()
        .expect("a land in a graveyard is a target");
    game.apply(PlayerId::One, activation).expect("it activates");
    settle(&mut game);

    assert!(
        game.players[0].graveyard.is_empty(),
        "the land left the graveyard",
    );
    assert_eq!(
        game.players[0]
            .exile
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::MOUNTAIN],
        "for exile",
    );
    assert_eq!(game.players[0].mana_pool.total(), 1, "and one mana arrived");
}

/// It targets, so it is not a mana ability however much mana it makes
/// (CR 605.1a): it is offered as an ordinary activation and waits on the
/// stack like one.
#[test]
fn it_is_not_a_mana_ability() {
    let (mut game, shaman) = staged(&[cards::MOUNTAIN], &[]);
    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::ActivateManaAbility { source, .. } if *source == shaman)),
        "no mana-ability shortcut for a targeted ability",
    );

    let activation = land_activations(&game, shaman)
        .into_iter()
        .next()
        .expect("a land in a graveyard is a target");
    game.apply(PlayerId::One, activation).expect("it activates");

    assert_eq!(game.stack.len(), 1, "it uses the stack");
    assert_eq!(
        game.players[0].mana_pool.total(),
        0,
        "and makes nothing until it resolves",
    );
}

/// "A graveyard" is either graveyard, and one with no land in it is no
/// target at all.
#[test]
fn it_reaches_their_graveyard_and_needs_a_land_in_one() {
    let (game, shaman) = staged(&[cards::LIGHTNING_BOLT], &[cards::FOREST]);
    assert_eq!(
        land_activations(&game, shaman).len(),
        1,
        "their Forest is a legal target and your Bolt is not",
    );

    let (game, shaman) = staged(&[cards::LIGHTNING_BOLT], &[cards::GRIZZLY_BEARS]);

    assert!(
        land_activations(&game, shaman).is_empty(),
        "no land in either graveyard is nothing to point at",
    );
}

/// Every way of activating her printed ability `index`.
fn activations(game: &Game, shaman: GameObjectId, index: u8) -> Vec<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(|action| {
            matches!(action, Action::ActivateAbility { source, ability, .. }
                if *source == shaman
                    && matches!(ability, AbilityOrigin::Printed { ability, .. }
                        if *ability == AbilityId(index)))
        })
        .collect()
}

/// The second ability: a black mana and a tap eats an instant or sorcery and
/// drains them for two.
#[test]
fn the_black_half_eats_a_spell_and_drains_them() {
    let (mut game, shaman) = staged(&[], &[cards::LIGHTNING_BOLT]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    let their_life = game.players[1].life;

    let action = activations(&game, shaman, 1)
        .into_iter()
        .next()
        .expect("a black mana and a tap");
    game.apply(PlayerId::One, action).expect("it activates");
    settle(&mut game);

    assert!(
        game.players[1]
            .exile
            .iter()
            .any(|card| card.definition == cards::LIGHTNING_BOLT),
        "their Bolt is exiled",
    );
    assert_eq!(game.players[1].life, their_life - 2, "and they lost two");
}

/// The third: a green mana and a tap eats a creature card and gains you two.
#[test]
fn the_green_half_eats_a_creature_and_gains_you_two() {
    let (mut game, shaman) = staged(&[cards::GRIZZLY_BEARS], &[]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 1);
    let my_life = game.players[0].life;

    let action = activations(&game, shaman, 2)
        .into_iter()
        .next()
        .expect("a green mana and a tap");
    game.apply(PlayerId::One, action).expect("it activates");
    settle(&mut game);

    assert!(
        game.players[0]
            .exile
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS),
        "the Bears are exiled",
    );
    assert_eq!(game.players[0].life, my_life + 2, "and you gained two");
}

/// Each half wants its own kind of card: a graveyard of nothing but lands
/// answers only the first ability, however much mana is available.
#[test]
fn each_half_names_only_its_own_kind_of_card() {
    let (mut game, shaman) = staged(&[cards::MOUNTAIN], &[]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 1);

    assert!(
        !activations(&game, shaman, 0).is_empty(),
        "the land ability has its land",
    );
    assert!(
        activations(&game, shaman, 1).is_empty(),
        "a Mountain is no instant or sorcery",
    );
    assert!(
        activations(&game, shaman, 2).is_empty(),
        "and it is no creature card either",
    );
}

/// "If the target ... is an illegal target when the ability tries to resolve,
/// it won't resolve and none of its effects will happen. You won't add mana,
/// no opponent will lose life, or you won't gain life, as appropriate."
#[test]
fn a_countered_activation_pays_out_neither_half() {
    let (mut game, shaman) = staged(&[], &[cards::LIGHTNING_BOLT]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    let their_life = game.players[1].life;
    let bolt = game.players[1].graveyard[0].id;

    let action = activations(&game, shaman, 1)
        .into_iter()
        .next()
        .expect("a black mana and a tap");
    game.apply(PlayerId::One, action).expect("it activates");

    // Answer it by removing what it named: the card leaves the graveyard
    // before the ability resolves.
    let card = remove_card(&mut game.players[1].graveyard, bolt).expect("it is there");
    game.players[1].hand.push(card);
    settle(&mut game);

    assert_eq!(
        game.players[1].life, their_life,
        "with its target gone the ability did nothing at all",
    );
    assert!(game.players[1].exile.is_empty(), "and nothing was exiled");
    assert!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == shaman)
            .is_some_and(|permanent| permanent.tapped),
        "though the tap and the mana are spent all the same",
    );
}
