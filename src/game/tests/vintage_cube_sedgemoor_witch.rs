//! Sedgemoor Witch: three power that is hard to block, hard to target, and
//! makes a body out of every cantrip.

use super::*;

/// The Witch on the battlefield since last turn, with `mine` in hand and
/// `theirs` in the other player's.
fn staged(mine: &[CardDefinitionId], theirs: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[1].hand.clear();
    let witch = game
        .put_onto_battlefield(PlayerId::One, cards::SEDGEMOOR_WITCH)
        .expect("cataloged");
    for (player, cards) in [(PlayerId::One, mine), (PlayerId::Two, theirs)] {
        for definition in cards {
            let card = game
                .build_zone(player, &[*definition])
                .expect("cataloged")
                .into_iter()
                .next()
                .expect("one card");
            game.players[player.index()].hand.push(card);
        }
    }
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turns_started = [4, 4];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.players[0].life = 20;
    game.players[1].life = 20;
    (game, witch)
}

fn settle(game: &mut Game) {
    for _ in 0..24 {
        if game.observe(PlayerId::One).decision.is_some()
            || game.observe(PlayerId::Two).decision.is_some()
        {
            return;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
    game.check_state_based_actions();
}

fn pests(game: &Game) -> usize {
    game.battlefield
        .iter()
        .filter(|permanent| is_token_with(permanent, tokens::pest()))
        .count()
}

/// Answers a waiting pay-or-counter decision, either paying or declining.
fn answer_ward(game: &mut Game, player: PlayerId, pay: bool) {
    let decision = game
        .observe(player)
        .decision
        .expect("the targeting player was asked about the ward cost");
    // The life cost names itself where a mana one says "Pay the cost".
    let label = if pay { "Pay 3 life" } else { "Decline" };
    let option = decision
        .options
        .iter()
        .find(|option| option.label == label)
        .unwrap_or_else(|| {
            panic!(
                "{label} is offered: {:?}",
                decision
                    .options
                    .iter()
                    .map(|option| option.label.clone())
                    .collect::<Vec<_>>()
            )
        })
        .id;
    game.apply(
        player,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![option],
        },
    )
    .expect("the answer is legal");
    settle(game);
}

/// Player Two bolts `target`, with the mana for it.
fn bolt(game: &mut Game, target: Target) {
    let card = game.players[1]
        .hand
        .iter()
        .find(|card| card.definition == cards::LIGHTNING_BOLT)
        .expect("the Bolt is in hand")
        .id;
    game.players[1].mana_pool.red = 1;
    game.priority = PlayerId::Two;
    game.apply(
        PlayerId::Two,
        cast_action(card, vec![target], Vec::new(), 0),
    )
    .expect("the Bolt is castable");
    settle(game);
}

/// Player One casts the instant or sorcery in hand.
fn cast_mine(game: &mut Game, definition: CardDefinitionId) {
    let card = game.players[0]
        .hand
        .iter()
        .find(|card| card.definition == definition)
        .expect("it is in hand")
        .id;
    for color in [ManaColor::Blue, ManaColor::Red, ManaColor::Black] {
        game.add_unrestricted_mana(PlayerId::One, color, 2);
    }
    game.priority = PlayerId::One;
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card: id, .. } if *id == card))
        .expect("there is mana for it");
    game.apply(PlayerId::One, action).expect("it is cast");
    settle(game);
}

/// Menace, which is the half of her that makes the other two matter.
#[test]
fn she_cannot_be_blocked_by_one_creature() {
    let (game, witch) = staged(&[], &[]);

    assert!(
        game.permanent_has_executable_keyword(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == witch)
                .expect("she is there"),
            KeywordAbility::Menace,
        ),
        "menace",
    );
}

/// Casting a sorcery makes a Pest, and the Pest pays a life back when it
/// dies.
#[test]
fn a_spell_makes_a_pest_that_pays_on_the_way_out() {
    let (mut game, _witch) = staged(&[cards::DURESS], &[]);
    assert_eq!(pests(&game), 0);

    cast_mine(&mut game, cards::DURESS);
    settle(&mut game);

    assert_eq!(pests(&game), 1, "magecraft made one");
    let pest = game
        .battlefield
        .iter()
        .find(|permanent| is_token_with(permanent, tokens::pest()))
        .expect("it is there")
        .card
        .id;
    let life = game.players[0].life;

    game.move_permanents_to_graveyard(&[pest]);
    settle(&mut game);

    assert_eq!(game.players[0].life, life + 1, "a life on the way out");
}

/// A creature spell is not an instant or a sorcery.
#[test]
fn a_creature_spell_makes_nothing() {
    let (mut game, _witch) = staged(&[cards::GRIZZLY_BEARS], &[]);

    for color in [ManaColor::Green, ManaColor::Blue] {
        game.add_unrestricted_mana(PlayerId::One, color, 2);
    }
    let bears = game.players[0].hand[0].id;
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card: id, .. } if *id == bears))
        .expect("there is mana for it");
    game.apply(PlayerId::One, action).expect("it is cast");
    settle(&mut game);

    assert_eq!(pests(&game), 0, "magecraft reads instant or sorcery");
}

/// Their removal is countered when they will not pay the three life.
#[test]
fn a_declined_ward_counters_their_spell() {
    let (mut game, witch) = staged(&[], &[cards::LIGHTNING_BOLT]);

    bolt(&mut game, Target::Permanent(witch));
    answer_ward(&mut game, PlayerId::Two, false);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == witch),
        "the Bolt never resolved",
    );
    assert_eq!(game.players[1].life, 20, "and it cost them nothing");
}

/// Paying is three life rather than three mana, which is the whole of what
/// her ward prints differently.
#[test]
fn a_paid_ward_costs_life() {
    let (mut game, witch) = staged(&[], &[cards::LIGHTNING_BOLT]);

    bolt(&mut game, Target::Permanent(witch));
    answer_ward(&mut game, PlayerId::Two, true);

    assert_eq!(game.players[1].life, 17, "three life for the privilege");
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == witch),
        "and three damage killed a 3/2",
    );
}

/// Ward reads "an opponent controls", so your own spells reach her for free.
#[test]
fn your_own_spell_pays_nothing() {
    let (mut game, witch) = staged(&[cards::LIGHTNING_BOLT], &[]);

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    let bolt = game.players[0].hand[0].id;
    game.apply(
        PlayerId::One,
        cast_action(bolt, vec![Target::Permanent(witch)], Vec::new(), 0),
    )
    .expect("the Bolt is castable");
    settle(&mut game);

    assert!(
        game.observe(PlayerId::One).decision.is_none(),
        "nobody was asked to pay",
    );
    assert_eq!(game.players[0].life, 20, "and nothing was paid");
    assert_eq!(pests(&game), 1, "the Bolt was still a spell she saw cast");
}

/// "If a player casts a spell that targets multiple permanents their
/// opponent controls with ward, each of those ward abilities will trigger.
/// If that player doesn't pay for all of them, the spell will be countered."
#[test]
fn two_wards_are_two_bills_and_one_unpaid_counters_it() {
    let (mut game, witch) = staged(&[], &[cards::FEELING_OF_DREAD]);
    let second = game
        .put_onto_battlefield(PlayerId::One, cards::SEDGEMOOR_WITCH)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    let dread = game.players[1].hand[0].id;
    game.players[1].mana_pool.white = 1;
    game.players[1].mana_pool.colorless = 1;
    game.priority = PlayerId::Two;
    game.apply(
        PlayerId::Two,
        cast_action(
            dread,
            vec![Target::Permanent(witch), Target::Permanent(second)],
            Vec::new(),
            0,
        ),
    )
    .expect("it may tap up to two creatures");

    // Both wards triggered at once, so their controller orders them first.
    let ordering = game
        .observe(PlayerId::One)
        .decision
        .expect("two triggers at once are ordered by the player who controls them");
    assert_eq!(ordering.options.len(), 2, "one ward apiece");
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: ordering.id,
            options: ordering.options.iter().map(|option| option.id).collect(),
        },
    )
    .expect("the order is theirs to pick");
    settle(&mut game);

    // One bill paid, one declined.
    answer_ward(&mut game, PlayerId::Two, true);
    answer_ward(&mut game, PlayerId::Two, false);
    settle(&mut game);

    assert_eq!(
        game.players[1].life, 17,
        "the ward they did pay cost them three",
    );
    assert!(
        game.battlefield
            .iter()
            .filter(|permanent| permanent.card.definition == cards::SEDGEMOOR_WITCH)
            .all(|permanent| !permanent.tapped),
        "and the spell was countered, so neither Witch was tapped",
    );
    assert!(
        game.players[1]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::FEELING_OF_DREAD),
        "the countered spell is in their graveyard",
    );
}
