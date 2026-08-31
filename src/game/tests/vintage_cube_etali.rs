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

/// Etali on the battlefield after both libraries are stacked, with the cast
/// offers her trigger hands out already declined.
fn staged(mine: &[CardDefinitionId], theirs: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let (mut game, etali) = staged_holding_offers(mine, theirs);
    drain_pending(&mut game);
    (game, etali)
}

/// The same, stopped at the standing cast offers rather than past them: the
/// permission the trigger hands out lives only while they wait.
fn staged_holding_offers(
    mine: &[CardDefinitionId],
    theirs: &[CardDefinitionId],
) -> (Game, GameObjectId) {
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
    drain_to_decision(&mut game);
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
/// Answers the standing offer in front of you by declining it, and stops at
/// the next one.
fn decline_offer(game: &mut Game) {
    let decision = game
        .pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("an offer is waiting");
    game.apply(
        decision.player,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![decision.options[0].id],
        },
    )
    .expect("declining is what the offer's one option is");
    drain_to_decision(game);
}

/// The exiled cards castable for nothing right now, by definition.
fn free_casts(game: &Game) -> Vec<CardDefinitionId> {
    let mut castable = game
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
    castable.sort_unstable();
    castable.dedup();
    castable
}

#[test]
fn their_card_is_yours_to_cast_for_free() {
    let (mut game, _etali) = staged_holding_offers(
        &[cards::MOUNTAIN, cards::LIGHTNING_BOLT],
        &[cards::GRIZZLY_BEARS],
    );

    assert_eq!(
        free_casts(&game),
        vec![cards::LIGHTNING_BOLT],
        "your own card is offered first",
    );
    assert_eq!(
        game.players[0].mana_pool.total(),
        0,
        "with no mana at all, which is what free means",
    );

    decline_offer(&mut game);
    assert_eq!(
        free_casts(&game),
        vec![cards::GRIZZLY_BEARS],
        "and then their creature, out of their own exile",
    );
}

/// "Any cards not cast remain in exile. They can't be cast on later turns."
/// The offer is the whole of the permission: decline it and the card is
/// stranded, on this turn as much as any later one.
#[test]
fn a_declined_card_is_stranded_in_exile() {
    let (mut game, _etali) = staged_holding_offers(
        &[cards::MOUNTAIN, cards::LIGHTNING_BOLT],
        &[cards::GRIZZLY_BEARS],
    );

    decline_offer(&mut game);
    decline_offer(&mut game);

    assert!(
        game.pending_decisions.is_empty(),
        "both offers have been answered",
    );
    assert!(
        free_casts(&game).is_empty(),
        "and nothing in the pile may be cast for the rest of the turn",
    );
    assert_eq!(
        exile_of(&game, PlayerId::One),
        vec![cards::MOUNTAIN, cards::LIGHTNING_BOLT],
        "the cards are still sitting there",
    );
    assert_eq!(exile_of(&game, PlayerId::Two), vec![cards::GRIZZLY_BEARS]);
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

/// "If you cast any of the exiled cards, you do so as part of the resolution
/// of the triggered ability... Timing restrictions based on a card's type
/// are ignored." A sorcery is offered mid-resolution, with a trigger on the
/// stack and nobody holding priority -- neither of which a sorcery survives
/// normally.
#[test]
fn a_sorcery_may_be_cast_while_the_trigger_is_resolving() {
    let (mut game, _etali) = staged_holding_offers(&[cards::WRATH_OF_GOD], &[cards::GRIZZLY_BEARS]);

    assert_eq!(
        free_casts(&game),
        vec![cards::WRATH_OF_GOD],
        "a sorcery is on offer inside somebody else's resolution",
    );

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, .. } => game.players[0]
                .exile
                .iter()
                .any(|exiled| exiled.id == *card && exiled.definition == cards::WRATH_OF_GOD),
            _ => false,
        })
        .expect("the Wrath is castable");
    game.apply(PlayerId::One, cast)
        .expect("it is cast for free");
    drain_pending(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::ETALI_PRIMAL_CONQUEROR),
        "and it resolves as a Wrath does, taking Etali with everything else",
    );
}

/// "If an exiled card has {X} in its mana cost, you must choose 0 as the
/// value of X when casting it without paying its mana cost." A Walking
/// Ballista cast for nothing is a 0/0 with no counters, which the game puts
/// straight back where it came from.
#[test]
fn an_x_cost_cast_for_free_is_cast_for_zero() {
    let (mut game, _etali) =
        staged_holding_offers(&[cards::WALKING_BALLISTA], &[cards::GRIZZLY_BEARS]);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                assert_eq!(choices.x(), 0, "no other X is on offer");
                game.players[0].exile.iter().any(|exiled| {
                    exiled.id == *card && exiled.definition == cards::WALKING_BALLISTA
                })
            }
            _ => false,
        })
        .expect("the Ballista is castable");
    game.apply(PlayerId::One, cast)
        .expect("it is cast for free");
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::WALKING_BALLISTA),
        "X was zero, so what arrived was a 0/0",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::WALKING_BALLISTA),
        "and a 0/0 does not stay on the battlefield",
    );
}
