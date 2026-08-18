//! Cards that tax or switch off what an opponent is doing.
//!
//! Each is symmetrical or unconditional in a way worth pinning down: Chill
//! taxes every red spell whoever casts it, Cursed Totem stops creature
//! abilities on both sides, and the Atog eats from either of two places.

use super::*;

fn ready() -> Game {
    let mut game = ready_game();
    game.turn = 5;
    game.turns_started[PlayerId::One.index()] = 5;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::One.index()].graveyard.clear();
    game
}

fn castable(game: &Game, id: GameObjectId) -> bool {
    game.legal_actions(PlayerId::One)
        .iter()
        .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == id))
}

fn settle(game: &mut Game) {
    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
}

/// Two more mana for a red spell, and nothing for anything else.
#[test]
fn chill_taxes_red_spells_only() {
    let mut game = ready();
    game.battlefield
        .push(creature(10_000, cards::CHILL, PlayerId::One));
    let bolt = card(20_000, cards::LIGHTNING_BOLT, PlayerId::One);
    let bolt_id = bolt.id;
    game.players[PlayerId::One.index()].hand.push(bolt);
    let recall = card(20_001, cards::ANCESTRAL_RECALL, PlayerId::One);
    let recall_id = recall.id;
    game.players[PlayerId::One.index()].hand.push(recall);
    // One red and one blue: enough for either spell unless the tax applies.
    game.players[PlayerId::One.index()].mana_pool.red = 1;
    game.players[PlayerId::One.index()].mana_pool.blue = 1;

    assert!(!castable(&game, bolt_id), "a Bolt now costs {{2}}{{R}}");
    assert!(castable(&game, recall_id), "and a blue spell is untouched");
}

/// Three more mana pays the tax off.
#[test]
fn chill_is_paid_off_with_the_extra_two() {
    let mut game = ready();
    game.battlefield
        .push(creature(10_000, cards::CHILL, PlayerId::One));
    let bolt = card(20_000, cards::LIGHTNING_BOLT, PlayerId::One);
    let bolt_id = bolt.id;
    game.players[PlayerId::One.index()].hand.push(bolt);
    game.players[PlayerId::One.index()].mana_pool.red = 1;
    game.players[PlayerId::One.index()].mana_pool.colorless = 2;

    assert!(castable(&game, bolt_id), "{{2}}{{R}} is payable");
}

/// The Totem stops creature abilities on both sides of the table, and leaves
/// noncreature abilities alone.
#[test]
fn the_totem_silences_creatures_on_both_sides() {
    let mut game = ready();
    game.battlefield
        .push(creature(10_000, cards::CURSED_TOTEM, PlayerId::One));
    let mine = creature(10_001, cards::GOBLIN_SHARPSHOOTER, PlayerId::One);
    let mine_id = mine.card.id;
    game.battlefield.push(mine);
    let theirs = creature(10_002, cards::GOBLIN_SHARPSHOOTER, PlayerId::Two);
    let theirs_id = theirs.card.id;
    game.battlefield.push(theirs);
    // A noncreature source with an activated ability, as the control.
    let factory = creature(10_003, cards::MISHRA_S_FACTORY, PlayerId::One);
    let factory_id = factory.card.id;
    game.battlefield.push(factory);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.players[PlayerId::One.index()].mana_pool.colorless = 4;

    let activates = |game: &Game, player: PlayerId, id: GameObjectId| {
        game.legal_actions(player)
            .iter()
            .any(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == id))
    };
    assert!(
        !activates(&game, PlayerId::One, mine_id),
        "your own creature is silenced too",
    );
    assert!(
        !activates(&game, PlayerId::Two, theirs_id),
        "and so is theirs",
    );
    assert!(
        activates(&game, PlayerId::One, factory_id),
        "a land is not a creature, so its ability still works",
    );
}

/// The Atog grows from hand or from graveyard, and the graveyard half spends
/// two cards for the same +1/+1.
#[test]
fn the_atog_eats_from_hand_and_from_graveyard() {
    let mut game = ready();
    let atog = creature(10_000, cards::PSYCHATOG, PlayerId::One);
    let atog_id = atog.card.id;
    game.battlefield.push(atog);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.players[PlayerId::One.index()].hand.push(card(
        20_000,
        cards::GRIZZLY_BEARS,
        PlayerId::One,
    ));
    for index in 0..2 {
        game.players[PlayerId::One.index()].graveyard.push(card(
            30_000 + index,
            cards::GRIZZLY_BEARS,
            PlayerId::One,
        ));
    }

    let feeds: Vec<_> = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == atog_id),
        )
        .collect();
    assert_eq!(
        feeds.len(),
        2,
        "one card in hand and one pair in the graveyard is one of each",
    );

    let stats = |game: &Game| {
        let permanent = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == atog_id)
            .expect("still there");
        (game.power(permanent), game.toughness(permanent))
    };
    assert_eq!(stats(&game), (Some(1), Some(2)), "a 1/2 to start");
    let feed = feeds.into_iter().next().expect("an activation is offered");
    game.apply(PlayerId::One, feed).expect("it is activated");
    settle(&mut game);
    assert_eq!(stats(&game), (Some(2), Some(3)), "and 2/3 after one meal");
}
