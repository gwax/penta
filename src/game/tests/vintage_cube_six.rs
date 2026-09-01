//! Six: a blocker that fills the graveyard and then plays out of it.

use super::*;

/// Six on the battlefield since last turn, with `library` stacked so the
/// last entry is on top and `graveyard` already in the graveyard.
fn staged(library: &[CardDefinitionId], graveyard: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    game.players[0].graveyard.clear();
    for definition in library {
        let card = game
            .build_zone(PlayerId::One, &[*definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        game.players[0].library.push(card);
    }
    for (index, definition) in graveyard.iter().enumerate() {
        game.players[0].graveyard.push(card(
            250_000 + u32::try_from(index).expect("few cards"),
            *definition,
            PlayerId::One,
        ));
    }
    let six = game
        .put_onto_battlefield(PlayerId::One, cards::SIX)
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
    (game, six)
}

/// Answers whatever is asked, taking the card whose definition is `wanted`
/// when it is on offer and nothing otherwise.
fn settle_taking(game: &mut Game, wanted: Option<CardDefinitionId>) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options: Vec<_> = decision
                .options
                .iter()
                .filter(|option| match (wanted, option.card) {
                    (Some(wanted), Some((_, ObjectCharacteristics::Card { definition, .. }))) => {
                        definition == wanted
                    }
                    _ => false,
                })
                .map(|option| option.id)
                .take(1)
                .collect();
            let options = if options.len() < decision.minimum {
                decision
                    .options
                    .iter()
                    .map(|option| option.id)
                    .take(decision.minimum)
                    .collect()
            } else {
                options
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
    game.check_state_based_actions();
}

fn attack_with(game: &mut Game, six: GameObjectId, wanted: Option<CardDefinitionId>) {
    game.step = Step::DeclareAttackers;
    game.declare_attacker(six, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    settle_taking(game, wanted);
}

fn castable(game: &Game, definition: CardDefinitionId) -> bool {
    let Some(card) = game.players[0]
        .graveyard
        .iter()
        .find(|card| card.definition == definition)
    else {
        return false;
    };
    game.legal_actions(PlayerId::One)
        .iter()
        .any(|action| matches!(action, Action::CastSpell { card: id, .. } if *id == card.id))
}

/// He has reach, which is what makes him a blocker worth attacking with.
#[test]
fn he_has_reach() {
    let (game, six) = staged(&[], &[]);
    let permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == six)
        .expect("he is there");

    assert!(game.permanent_has_executable_keyword(permanent, KeywordAbility::Reach));
}

/// Attacking mills three and offers the land among them.
#[test]
fn attacking_mills_three_and_finds_a_land() {
    let (mut game, six) = staged(
        &[
            cards::LIGHTNING_BOLT,
            cards::MOUNTAIN,
            cards::GRIZZLY_BEARS,
            cards::SERRA_ANGEL,
        ],
        &[],
    );

    attack_with(&mut game, six, Some(cards::MOUNTAIN));

    assert_eq!(
        game.players[0]
            .hand
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::MOUNTAIN],
        "the land came back to hand",
    );
    assert_eq!(
        game.players[0].graveyard.len(),
        2,
        "and the other two stayed milled",
    );
    assert_eq!(game.players[0].library.len(), 1);
}

/// Taking nothing is a legal answer.
#[test]
fn the_land_may_be_left_behind() {
    let (mut game, six) = staged(
        &[
            cards::LIGHTNING_BOLT,
            cards::MOUNTAIN,
            cards::GRIZZLY_BEARS,
            cards::SERRA_ANGEL,
        ],
        &[],
    );

    attack_with(&mut game, six, None);

    assert!(game.players[0].hand.is_empty());
    assert_eq!(game.players[0].graveyard.len(), 3);
}

/// On your turn a nonland permanent card in the graveyard may be cast by
/// discarding a land; an instant may not, and a land may not.
#[test]
fn your_turn_grants_retrace_to_permanent_cards() {
    let (mut game, _six) = staged(&[], &[cards::GRIZZLY_BEARS, cards::LIGHTNING_BOLT]);
    game.players[0]
        .hand
        .push(card(250_100, cards::MOUNTAIN, PlayerId::One));
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 2);

    assert!(
        castable(&game, cards::GRIZZLY_BEARS),
        "a creature card in the graveyard has retrace",
    );
    assert!(
        !castable(&game, cards::LIGHTNING_BOLT),
        "an instant is not a permanent card",
    );
}

/// And only on your turn.
#[test]
fn their_turn_grants_nothing() {
    let (mut game, _six) = staged(&[], &[cards::GRIZZLY_BEARS]);
    game.players[0]
        .hand
        .push(card(250_200, cards::MOUNTAIN, PlayerId::One));
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 2);
    game.active_player = PlayerId::Two;
    game.step = Step::End;
    game.priority = PlayerId::One;

    assert!(!castable(&game, cards::GRIZZLY_BEARS));
}

/// Casting one that way discards the land and leaves the card where retrace
/// leaves it: back in the graveyard.
#[test]
fn a_retraced_permanent_costs_a_land_from_hand() {
    let (mut game, _six) = staged(&[], &[cards::GRIZZLY_BEARS]);
    game.players[0]
        .hand
        .push(card(250_300, cards::MOUNTAIN, PlayerId::One));
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 2);

    let bears = game.players[0].graveyard[0].id;
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == bears))
        .expect("retrace is payable");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle_taking(&mut game, None);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::GRIZZLY_BEARS),
        "the creature arrived",
    );
    assert_eq!(
        game.players[0]
            .graveyard
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::MOUNTAIN],
        "and the land it cost is what is left in the graveyard",
    );
    assert!(game.players[0].hand.is_empty());
}

/// Retrace is not flashback: the card is not exiled, so a countered spell
/// lands back in the graveyard and the permission still reaches it. Only
/// another land is needed to pay for the second try.
#[test]
fn a_countered_retrace_may_be_retraced_again() {
    let (mut game, _six) = staged(&[], &[cards::GRIZZLY_BEARS]);
    game.players[0]
        .hand
        .push(card(250_300, cards::MOUNTAIN, PlayerId::One));
    game.players[0]
        .hand
        .push(card(250_301, cards::MOUNTAIN, PlayerId::One));
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 2);

    let counterspell = card(250_302, cards::COUNTERSPELL, PlayerId::Two);
    let counterspell_id = counterspell.id;
    game.players[1].hand.push(counterspell);
    game.players[1].mana_pool.blue = 2;

    let bears = game.players[0].graveyard[0].id;
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == bears))
        .expect("retrace is payable");
    game.apply(PlayerId::One, cast).expect("it is cast");
    acceptance_attempt_counterspell(&mut game, counterspell_id);
    settle_taking(&mut game, None);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 2);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::GRIZZLY_BEARS),
        "the countered spell never arrived",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS),
        "it went back to the graveyard rather than into exile",
    );
    assert!(
        game.players[0]
            .exile
            .iter()
            .all(|card| card.definition != cards::GRIZZLY_BEARS),
        "retrace exiles nothing",
    );
    assert!(
        castable(&game, cards::GRIZZLY_BEARS),
        "and the second land in hand pays for another try",
    );
}

/// "A land card from among them" is the three the attack just milled. A
/// land that was already in the graveyard is not among them, so an attack
/// that mills nothing but spells finds nothing to take.
#[test]
fn the_land_must_come_from_the_three_that_were_milled() {
    let (mut game, six) = staged(
        &[
            cards::LIGHTNING_BOLT,
            cards::LIGHTNING_BOLT,
            cards::LIGHTNING_BOLT,
        ],
        &[cards::MOUNTAIN],
    );

    attack_with(&mut game, six, Some(cards::MOUNTAIN));

    assert!(
        game.players[0].hand.is_empty(),
        "the Mountain already in the graveyard was not on offer",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::MOUNTAIN),
        "it is where it was",
    );
    assert_eq!(
        game.players[0]
            .graveyard
            .iter()
            .filter(|card| card.definition == cards::LIGHTNING_BOLT)
            .count(),
        3,
        "and the three were milled all the same",
    );
}

/// "Nonland permanent cards in your graveyard": the pile the permission
/// reaches is its controller's own. What is in theirs stays there.
#[test]
fn the_permission_does_not_reach_their_graveyard() {
    let (mut game, _six) = staged(&[], &[]);
    game.players[1].graveyard.clear();
    game.players[1]
        .graveyard
        .push(card(250_400, cards::GRIZZLY_BEARS, PlayerId::Two));
    let theirs = game.players[1].graveyard[0].id;
    game.players[0]
        .hand
        .push(card(250_401, cards::MOUNTAIN, PlayerId::One));
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 2);

    assert!(
        game.legal_actions(PlayerId::One)
            .iter()
            .all(|action| !matches!(action, Action::CastSpell { card, .. } if *card == theirs)),
        "their graveyard is not yours to retrace out of",
    );

    // The same card in your own graveyard, to show the mana and the land
    // were there for it all along.
    game.players[0]
        .graveyard
        .push(card(250_402, cards::GRIZZLY_BEARS, PlayerId::One));
    assert!(
        castable(&game, cards::GRIZZLY_BEARS),
        "yours is castable from the same position",
    );
}
