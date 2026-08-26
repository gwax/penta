//! Dragon's Rage Channeler: a one-mana 1/1 that fills its own graveyard and
//! becomes a 3/3 flier for doing what the deck was doing anyway.

use super::*;

/// The Channeler on the battlefield since last turn, with `graveyard` in
/// Player One's graveyard and `library` on top of their library.
fn staged(graveyard: &[CardDefinitionId], library: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    game.players[0].graveyard.clear();
    for (index, definition) in graveyard.iter().enumerate() {
        game.players[0].graveyard.push(card(
            109_000 + u32::try_from(index).expect("a small graveyard"),
            *definition,
            PlayerId::One,
        ));
    }
    for definition in library {
        let card = game
            .build_zone(PlayerId::One, &[*definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        game.players[0].library.push(card);
    }
    let channeler = game
        .put_onto_battlefield(PlayerId::One, cards::DRAGON_S_RAGE_CHANNELER)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, channeler)
}

fn permanent(game: &Game, id: GameObjectId) -> &Permanent {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("it is on the battlefield")
}

/// Casts `definition` from Player One's hand with mana to spare.
fn cast(game: &mut Game, definition: CardDefinitionId) {
    let card = game
        .build_zone(PlayerId::One, &[definition])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let held = card.id;
    game.players[0].hand.push(card);
    for color in [ManaColor::Red, ManaColor::Green, ManaColor::Colorless] {
        game.add_unrestricted_mana(PlayerId::One, color, 3);
    }
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == held))
        .expect("it is castable");
    game.apply(PlayerId::One, action).expect("it is cast");
    settle(game);
}

/// Answers the surveil by putting the card into the graveyard when
/// `bury` is set, and leaving it on top otherwise.
fn settle_surveil(game: &mut Game, bury: bool) {
    for _ in 0..16 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = if bury {
                decision
                    .options
                    .iter()
                    .map(|option| option.id)
                    .take(1)
                    .collect()
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
    drain_pending(game);
}

fn settle(game: &mut Game) {
    settle_surveil(game, true);
}

/// Casting a noncreature spell surveils: the top card may be buried.
#[test]
fn a_noncreature_spell_surveils() {
    let (mut game, _) = staged(&[], &[cards::MOUNTAIN, cards::GRIZZLY_BEARS]);

    cast(&mut game, cards::LIGHTNING_BOLT);

    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS),
        "the top card went to the graveyard",
    );
    assert_eq!(game.players[0].library.len(), 1, "and the rest stayed");
}

/// The look may leave the card on top instead.
#[test]
fn the_surveil_may_keep_the_card() {
    let (mut game, _) = staged(&[], &[cards::MOUNTAIN, cards::GRIZZLY_BEARS]);
    let card = game
        .build_zone(PlayerId::One, &[cards::LIGHTNING_BOLT])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let held = card.id;
    game.players[0].hand.push(card);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 3);
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == held))
        .expect("it is castable");
    game.apply(PlayerId::One, action).expect("it is cast");
    settle_surveil(&mut game, false);

    assert_eq!(game.players[0].library.len(), 2, "nothing was buried");
    assert!(
        !game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS),
    );
}

/// A creature spell is not a noncreature spell, so nothing is surveilled.
#[test]
fn a_creature_spell_surveils_nothing() {
    let (mut game, _) = staged(&[], &[cards::MOUNTAIN, cards::ISLAND]);

    cast(&mut game, cards::GRIZZLY_BEARS);

    assert_eq!(game.players[0].library.len(), 2, "the library is untouched");
}

/// Three card types is not delirium: a 1/1 on the ground.
#[test]
fn three_card_types_leave_it_a_one_one() {
    let (game, channeler) = staged(
        &[cards::MOUNTAIN, cards::GRIZZLY_BEARS, cards::LIGHTNING_BOLT],
        &[],
    );

    let body = permanent(&game, channeler);
    assert_eq!(game.power(body), Some(1));
    assert_eq!(game.toughness(body), Some(1));
    assert!(!game.has_flying(body));
}

/// Four card types turn it into a 3/3 flier that has to attack.
#[test]
fn delirium_makes_it_a_three_three_flier() {
    let (game, channeler) = staged(
        &[
            cards::MOUNTAIN,
            cards::GRIZZLY_BEARS,
            cards::LIGHTNING_BOLT,
            cards::BLACK_LOTUS,
        ],
        &[],
    );

    let body = permanent(&game, channeler);
    assert_eq!(game.power(body), Some(3));
    assert_eq!(game.toughness(body), Some(3));
    assert!(game.has_flying(body), "and it flies");
    assert!(
        game.permanent_has_executable_keyword(body, KeywordAbility::AttacksEachCombatIfAble),
        "which is what the compulsion to attack pays for",
    );
}

/// "As long as": the delirium leaves with the fourth card type.
#[test]
fn losing_the_fourth_type_takes_it_all_back() {
    let (mut game, channeler) = staged(
        &[
            cards::MOUNTAIN,
            cards::GRIZZLY_BEARS,
            cards::LIGHTNING_BOLT,
            cards::BLACK_LOTUS,
        ],
        &[],
    );
    assert_eq!(game.power(permanent(&game, channeler)), Some(3));

    game.players[0]
        .graveyard
        .retain(|card| card.definition != cards::BLACK_LOTUS);

    let body = permanent(&game, channeler);
    assert_eq!(game.power(body), Some(1), "back to a 1/1");
    assert!(!game.has_flying(body), "and back on the ground");
    assert!(
        !game.permanent_has_executable_keyword(body, KeywordAbility::AttacksEachCombatIfAble),
        "and free to stay home",
    );
}
