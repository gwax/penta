//! Crabomination: the six mana printed on it is not the price anybody pays.
//! An artifact that has already done its work pays most of it.

use super::*;

/// Player One holding Crabomination with `artifacts` on the battlefield.
fn staged(artifacts: &[CardDefinitionId]) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let mut ids = Vec::new();
    for definition in artifacts {
        ids.push(
            game.put_onto_battlefield(PlayerId::One, *definition)
                .expect("cataloged"),
        );
    }
    drain_pending(&mut game);
    let card = game
        .build_zone(PlayerId::One, &[cards::CRABOMINATION])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let held = card.id;
    game.players[0].hand.push(card);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, held, ids)
}

/// Every way Crabomination can be cast right now.
fn casts(game: &Game, held: GameObjectId) -> Vec<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(|action| matches!(action, Action::CastSpell { card, .. } if *card == held))
        .collect()
}

/// The emerge casts, and what each of them sacrifices.
fn emerges(game: &Game, held: GameObjectId) -> Vec<(Action, Vec<GameObjectId>)> {
    casts(game, held)
        .into_iter()
        .filter_map(|action| match &action {
            Action::CastSpell {
                choices,
                sacrifices,
                ..
            } if choices.costs().alternative().is_some() => {
                let spent = sacrifices.clone();
                Some((action, spent))
            }
            _ => None,
        })
        .collect()
}

/// A two-mana artifact takes two off the emerge cost, leaving five mana.
#[test]
fn the_artifacts_mana_value_pays_for_it() {
    let (mut game, held, artifacts) = staged(&[cards::LIGHTNING_GREAVES]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 2);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 3);

    let offered = emerges(&game, held);
    assert_eq!(offered.len(), 1, "one artifact, one way to emerge");
    assert_eq!(offered[0].1, vec![artifacts[0]], "the Greaves pay");
    game.apply(PlayerId::One, offered[0].0.clone())
        .expect("five mana and the Greaves is the whole cost");
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::CRABOMINATION),
        "the 5/5 arrived",
    );
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == artifacts[0]),
        "and the Greaves were sacrificed to pay for it",
    );
    assert_eq!(game.players[0].mana_pool.total(), 0, "all five were spent");
}

/// One mana short is one mana short: five mana does not cast it off a
/// two-mana artifact.
#[test]
fn the_reduction_is_only_the_artifacts_mana_value() {
    let (mut game, held, _) = staged(&[cards::LIGHTNING_GREAVES]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 2);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);

    assert!(
        emerges(&game, held).is_empty(),
        "four mana plus a two-drop is one short of the emerge cost",
    );
}

/// The reduction is generic only: a big colourless artifact does not pay
/// the two black pips.
#[test]
fn the_black_pips_are_still_owed() {
    let (mut game, held, _) = staged(&[cards::BLIGHTSTEEL_COLOSSUS]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 7);

    assert!(
        emerges(&game, held).is_empty(),
        "a twelve-drop covers the generic and none of the black",
    );

    game.players[0].mana_pool = ManaPool::default();
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 2);
    let offered = emerges(&game, held);
    assert_eq!(offered.len(), 1, "two black is the whole of what is left");
    game.apply(PlayerId::One, offered[0].0.clone())
        .expect("it casts");
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::CRABOMINATION),
        "the Crab arrived for two black mana and a Colossus",
    );
}

/// Without an artifact there is nothing to emerge from, and the ordinary
/// six-mana cast is the only one on offer.
#[test]
fn no_artifact_means_no_emerge() {
    let (mut game, held, _) = staged(&[]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 2);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 4);

    assert!(emerges(&game, held).is_empty(), "nothing to sacrifice");
    assert_eq!(casts(&game, held).len(), 1, "but it is still a spell");
}

/// Each artifact is its own way to emerge, priced by its own mana value.
#[test]
fn each_artifact_is_its_own_offer() {
    let (mut game, held, artifacts) = staged(&[cards::MIND_STONE, cards::BLACK_LOTUS]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 2);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 5);

    let offered = emerges(&game, held);
    assert_eq!(offered.len(), 2, "one offer per artifact");
    let spent = offered
        .iter()
        .map(|(_, sacrifices)| sacrifices.clone())
        .collect::<Vec<_>>();
    assert!(spent.contains(&vec![artifacts[0]]));
    assert!(spent.contains(&vec![artifacts[1]]));
}

/// An artifact that makes mana may do both: tapped for its mana first, and
/// sacrificed to emerge afterwards.
#[test]
fn a_mana_rock_pays_twice() {
    let (mut game, held, artifacts) = staged(&[cards::MIND_STONE]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 2);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);

    let offered = emerges(&game, held);
    assert_eq!(
        offered.len(),
        1,
        "four mana in the pool and one more out of the Stone itself",
    );
    game.apply(PlayerId::One, offered[0].0.clone())
        .expect("the Stone is tapped for mana and then sacrificed");
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::CRABOMINATION),
        "the Crab arrived",
    );
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == artifacts[0]),
        "and the Stone is gone",
    );
}

/// Their library, graveyard, and hand each lose one card to exile, and the
/// Crab's controller may cast one of them for nothing.
fn staged_with_their_cards() -> (Game, GameObjectId) {
    let (mut game, held, _) = staged(&[]);
    game.players[1].hand.clear();
    game.players[1].graveyard.clear();
    game.players[1].library.clear();
    game.players[1]
        .library
        .push(card(107_000, cards::GRIZZLY_BEARS, PlayerId::Two));
    game.players[1]
        .graveyard
        .push(card(107_001, cards::LIGHTNING_BOLT, PlayerId::Two));
    game.players[1]
        .hand
        .push(card(107_002, cards::SAVANNAH_LIONS, PlayerId::Two));
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 2);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 4);
    (game, held)
}

/// Casts the Crab for its printed cost and lets the trigger resolve, past
/// the cast offer it hands out.
fn hard_cast(game: &mut Game, held: GameObjectId) {
    hard_cast_holding_offer(game, held);
    drain_pending(game);
}

/// The same, stopped at the standing offer: the free cast happens while it
/// waits or not at all.
fn hard_cast_holding_offer(game: &mut Game, held: GameObjectId) {
    let action = casts(game, held)
        .into_iter()
        .next()
        .expect("six mana casts it");
    game.apply(PlayerId::One, action).expect("it is cast");
    drain_to_decision(game);
}

/// One card out of each of the three zones.
#[test]
fn it_takes_one_card_from_each_zone() {
    let (mut game, held) = staged_with_their_cards();

    hard_cast(&mut game, held);

    assert!(game.players[1].library.is_empty(), "the top card is gone");
    assert!(game.players[1].graveyard.is_empty(), "and the graveyard");
    assert!(game.players[1].hand.is_empty(), "and the hand");
    let exiled = game.players[1]
        .exile
        .iter()
        .map(|card| card.definition)
        .collect::<Vec<_>>();
    assert_eq!(exiled.len(), 3, "all three are in exile");
    for definition in [
        cards::GRIZZLY_BEARS,
        cards::LIGHTNING_BOLT,
        cards::SAVANNAH_LIONS,
    ] {
        assert!(exiled.contains(&definition), "{definition:?} was exiled");
    }
}

/// A zone with nothing in it gives nothing, and the others still give.
#[test]
fn empty_zones_are_skipped() {
    let (mut game, held) = staged_with_their_cards();
    game.players[1].graveyard.clear();
    game.players[1].hand.clear();

    hard_cast(&mut game, held);

    assert_eq!(
        game.players[1].exile.len(),
        1,
        "only the library had a card to give",
    );
}

/// The exiled cards castable for nothing right now.
fn free_casts(game: &Game) -> Vec<GameObjectId> {
    let mut free = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell { card, .. }
                if game.players[1].exile.iter().any(|exiled| exiled.id == card) =>
            {
                Some(card)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    free.sort_unstable();
    free.dedup();
    free
}

/// Declines the offer in front of you and stops at the next one.
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

/// The permission is the Crab's controller's, and it is a choice among the
/// pile rather than a queue: all three of their cards stand on offer, and
/// the one that is cast is whichever the caster names.
#[test]
fn one_of_the_three_may_be_cast_for_free() {
    let (mut game, held) = staged_with_their_cards();
    hard_cast_holding_offer(&mut game, held);
    game.players[0].mana_pool = ManaPool::default();

    let free = free_casts(&game);
    assert_eq!(free.len(), 3, "all three are on offer, with no mana at all");
    assert!(
        free.iter().all(|card| game.players[1]
            .exile
            .iter()
            .any(|exiled| exiled.id == *card)),
        "and every one of them is a card out of their zones",
    );

    // The last of them, which a queue would have reached only after two
    // declines -- and a decline takes the whole pile's permission.
    let last = *free.last().expect("three of them");
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == last))
        .expect("the offer behind the others is answerable too");
    game.apply(PlayerId::One, cast)
        .expect("it is cast for free");
    drain_pending(&mut game);

    assert!(
        !game.players[1].exile.iter().any(|exiled| exiled.id == last),
        "the card the caster picked is the one that left exile",
    );
    assert_eq!(
        game.players[1].exile.len(),
        2,
        "and the other two stayed where they were",
    );
}

/// Declining is an answer like any other: the pile's one permission goes
/// back, and the ability has resolved, so nothing in it may be cast later in
/// the turn.
#[test]
fn declining_strands_the_whole_pile() {
    let (mut game, held) = staged_with_their_cards();
    hard_cast_holding_offer(&mut game, held);
    game.players[0].mana_pool = ManaPool::default();

    decline_offer(&mut game);
    drain_pending(&mut game);

    assert!(
        free_casts(&game).is_empty(),
        "what was not cast as it resolved stays in exile",
    );
    assert_eq!(game.players[1].exile.len(), 3, "all three are still there");
}

/// "A spell", not all of them: casting one spends the permission over the
/// whole pile, and the offers behind it go with it.
#[test]
fn casting_one_spends_the_permission() {
    let (mut game, held) = staged_with_their_cards();
    hard_cast_holding_offer(&mut game, held);
    game.players[0].mana_pool = ManaPool::default();

    let first = *free_casts(&game).first().expect("one of them is on offer");
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == first))
        .expect("it is free to cast");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);

    assert!(
        free_casts(&game).is_empty(),
        "the rest of the pile is no longer castable",
    );
}
