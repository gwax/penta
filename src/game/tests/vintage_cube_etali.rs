//! Etali, Primal Conqueror: seven mana that casts the two best cards on the
//! table, and a back face nobody in the cube ever pays for.

use super::*;

/// Every way of activating Etali's transform ability that is on offer.
fn transforms(game: &Game, etali: GameObjectId) -> Vec<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == etali),
        )
        .collect()
}

/// Etali on the battlefield after both libraries are stacked.
fn staged(mine: &[CardDefinitionId], theirs: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    game.players[1].library.clear();
    for (player, cards) in [(PlayerId::One, mine), (PlayerId::Two, theirs)] {
        // The library reads from the back, so these are stacked top last.
        for definition in cards.iter().rev() {
            let card = game
                .build_zone(player, &[*definition])
                .expect("cataloged")
                .into_iter()
                .next()
                .expect("one card");
            game.players[player.index()].library.push(card);
        }
    }
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    let etali = game
        .put_onto_battlefield(PlayerId::One, cards::ETALI_PRIMAL_CONQUEROR)
        .expect("cataloged");
    drain_pending(&mut game);
    (game, etali)
}

fn exile_of(game: &Game, player: PlayerId) -> Vec<CardDefinitionId> {
    game.players[player.index()]
        .exile
        .iter()
        .map(|card| card.definition)
        .collect()
}

/// It digs through both libraries until each turns up a nonland card.
#[test]
fn it_exiles_from_both_libraries_until_a_nonland() {
    let (game, _etali) = staged(
        &[cards::MOUNTAIN, cards::LIGHTNING_BOLT, cards::ISLAND],
        &[cards::FOREST, cards::FOREST, cards::GRIZZLY_BEARS],
    );

    assert_eq!(
        exile_of(&game, PlayerId::One),
        vec![cards::MOUNTAIN, cards::LIGHTNING_BOLT],
        "the land it walked past is exiled too",
    );
    assert_eq!(
        exile_of(&game, PlayerId::Two),
        vec![cards::FOREST, cards::FOREST, cards::GRIZZLY_BEARS],
        "and it digs as deep as their library makes it",
    );
    assert_eq!(
        game.players[0].library.len(),
        1,
        "it stops at the first nonland",
    );
}

/// The nonland card each library turned up may be cast for nothing, and the
/// permission is Etali's controller's even for their card.
#[test]
fn their_card_is_yours_to_cast_for_free() {
    let (game, _etali) = staged(
        &[cards::MOUNTAIN, cards::LIGHTNING_BOLT],
        &[cards::GRIZZLY_BEARS],
    );

    let castable = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell { card, .. } => game.players[1]
                .exile
                .iter()
                .chain(game.players[0].exile.iter())
                .find(|exiled| exiled.id == card)
                .map(|exiled| exiled.definition),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(
        castable.contains(&cards::GRIZZLY_BEARS),
        "their creature is castable out of their own exile: {castable:?}",
    );
    assert!(
        castable.contains(&cards::LIGHTNING_BOLT),
        "and so is yours: {castable:?}",
    );
    assert_eq!(
        game.players[0].mana_pool.total(),
        0,
        "with no mana at all, which is what free means",
    );
}

/// A land it walked past is not castable: only the card that stopped the
/// dig carries the permission.
#[test]
fn the_lands_it_passed_are_not_castable() {
    let (game, _etali) = staged(&[cards::MOUNTAIN, cards::LIGHTNING_BOLT], &[cards::ISLAND]);
    let mountain = game.players[0]
        .exile
        .iter()
        .find(|card| card.definition == cards::MOUNTAIN)
        .expect("the Mountain was exiled")
        .id;

    assert!(
        !game.legal_actions(PlayerId::One).iter().any(|action| {
            matches!(action, Action::PlayLand { card, .. } | Action::CastSpell { card, .. }
                if *card == mountain)
        }),
        "the land it passed stays in exile",
    );
}

/// The transform is sorcery speed, and what it leaves is an 11/11 that
/// poisons. The Phyrexian pip is paid with green mana here, which is the
/// activation that announces nothing.
#[test]
fn it_transforms_into_the_back_face() {
    let (mut game, etali) = staged(&[cards::LIGHTNING_BOLT], &[cards::GRIZZLY_BEARS]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 9);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 1);

    let transform = transforms(&game, etali)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { mana_payment, .. } if mana_payment.is_none())
        })
        .expect("ten mana turns it over");
    game.apply(PlayerId::One, transform).expect("it activates");
    drain_pending(&mut game);

    let back = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == etali)
        .expect("it is still there");
    assert_eq!(game.power(back), Some(11), "an 11/11");
    assert_eq!(game.toughness(back), Some(11));
    assert!(game.permanent_has_executable_keyword(back, KeywordAbility::Indestructible));
    assert!(game.permanent_has_executable_keyword(back, KeywordAbility::Trample));
    assert_eq!(game.players[0].life, 20, "paid with mana rather than life");
}

/// The back face's combat damage is poison rather than life loss.
#[test]
fn the_back_face_poisons_what_it_hits() {
    let (mut game, etali) = staged(&[cards::LIGHTNING_BOLT], &[cards::GRIZZLY_BEARS]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 9);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 1);
    let transform = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == etali))
        .expect("the transform is offered");
    game.apply(PlayerId::One, transform).expect("it activates");
    drain_pending(&mut game);
    let life = game.players[1].life;

    game.damage_target_from_kind(Some(etali), Some(Target::Player(PlayerId::Two)), 11, true);
    drain_pending(&mut game);

    assert_eq!(
        game.players[1].counters.count(CounterKind::Poison),
        11,
        "eleven poison counters",
    );
    assert_eq!(
        game.players[1].life,
        life - 11,
        "and the damage still happened",
    );
}

/// "{9}{G/P}": the Phyrexian pip is payable with two life instead of green
/// mana, so nine colourless and a pulse is enough to turn it over.
#[test]
fn two_life_pays_the_phyrexian_pip() {
    let (mut game, etali) = staged(&[cards::LIGHTNING_BOLT], &[cards::GRIZZLY_BEARS]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 9);

    let offered = transforms(&game, etali);
    assert_eq!(
        offered.len(),
        1,
        "with no green anywhere, life is the only way to pay the pip",
    );
    game.apply(
        PlayerId::One,
        offered.into_iter().next().expect("one offer"),
    )
    .expect("it activates");
    drain_pending(&mut game);

    assert_eq!(game.players[0].life, 18, "two life for the pip");
    let back = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == etali)
        .expect("it is still there");
    assert_eq!(game.power(back), Some(11), "and it turned over");
    assert_eq!(
        game.players[0].mana_pool.total(),
        0,
        "the nine generic was still paid with mana",
    );
}

/// With the green mana available, both ways of paying are on offer: the
/// branch is the player's to announce, not the engine's to pick.
#[test]
fn both_ways_of_paying_the_pip_are_offered() {
    let (game, etali) = staged(&[cards::LIGHTNING_BOLT], &[cards::GRIZZLY_BEARS]);
    let mut game = game;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 9);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 1);

    let offered = transforms(&game, etali);

    assert_eq!(offered.len(), 2, "green mana, or two life");
    assert_eq!(
        offered
            .iter()
            .filter(|action| {
                matches!(action, Action::ActivateAbility { mana_payment, .. } if mana_payment.is_some())
            })
            .count(),
        1,
        "exactly one of them announces paying with life",
    );
}

/// A player at two life may still pay it; one at one life may not (CR
/// 118.4), and with no green mana that leaves the ability unactivatable.
#[test]
fn a_player_too_low_on_life_cannot_pay_the_pip() {
    let (mut game, etali) = staged(&[cards::LIGHTNING_BOLT], &[cards::GRIZZLY_BEARS]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 9);
    game.players[0].life = 2;
    assert_eq!(transforms(&game, etali).len(), 1, "two life is payable");

    game.players[0].life = 1;

    assert!(
        transforms(&game, etali).is_empty(),
        "one life does not pay a two-life pip",
    );
}
