//! Wrath of God: what a sweeper takes, and what it leaves standing.

use super::*;

/// Casts the Wrath from Player One's hand and lets it resolve.
fn wrath(game: &mut Game) {
    let spell = game
        .build_zone(PlayerId::One, &[cards::WRATH_OF_GOD])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let spell_id = spell.id;
    game.players[0].hand.push(spell);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 2);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);
    game.priority = PlayerId::One;
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == spell_id))
        .expect("four mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(game);
    game.check_state_based_actions();
}

fn on_battlefield(game: &Game, definition: CardDefinitionId) -> bool {
    game.battlefield
        .iter()
        .any(|permanent| permanent.card.definition == definition)
}

/// "All creatures" reaches across the table, and it stops at creatures: the
/// artifact beside them is untouched, and so is the one that cannot be
/// destroyed.
#[test]
fn it_sweeps_both_boards_and_leaves_the_rest() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    for definition in [cards::GRIZZLY_BEARS, cards::SOL_RING] {
        game.put_onto_battlefield(PlayerId::One, definition)
            .expect("cataloged");
    }
    for definition in [cards::SERRA_ANGEL, cards::BLIGHTSTEEL_COLOSSUS] {
        game.put_onto_battlefield(PlayerId::Two, definition)
            .expect("cataloged");
    }
    drain_pending(&mut game);

    wrath(&mut game);

    assert!(!on_battlefield(&game, cards::GRIZZLY_BEARS), "yours died");
    assert!(!on_battlefield(&game, cards::SERRA_ANGEL), "and theirs did");
    assert!(
        on_battlefield(&game, cards::SOL_RING),
        "an artifact is not a creature",
    );
    assert!(
        on_battlefield(&game, cards::BLIGHTSTEEL_COLOSSUS),
        "and destroy does nothing to what cannot be destroyed",
    );
}

/// A token that is destroyed ceases to exist rather than going anywhere, and
/// the cards that died are in their owners' graveyards -- each in their own.
#[test]
fn what_it_kills_goes_to_its_own_owners_graveyard() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    game.players[1].graveyard.clear();
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    // Theirs, but under your control: the card goes home to its owner.
    let mut stolen = creature(120_000, cards::SAVANNAH_LIONS, PlayerId::Two);
    stolen.controller = PlayerId::One;
    game.battlefield.push(stolen);
    game.put_onto_battlefield(PlayerId::Two, cards::GRAVE_TITAN)
        .expect("cataloged");
    drain_pending(&mut game);
    assert_eq!(
        game.battlefield.len(),
        4,
        "the Titan brought two Zombies with it",
    );

    wrath(&mut game);

    assert!(game.battlefield.is_empty(), "nothing survived");
    assert_eq!(
        game.players[1]
            .graveyard
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::SAVANNAH_LIONS, cards::GRAVE_TITAN],
        "both cards are in their owner's graveyard, whoever controlled them, \
         and the two Zombie tokens left nothing behind at all",
    );
    assert_eq!(
        game.players[0]
            .graveyard
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::WRATH_OF_GOD],
        "and the only thing in yours is the spell you cast",
    );
}

/// "They can't be regenerated": the second sentence, which is the whole
/// difference between this and a Damnation-shaped sweeper of its era. A
/// Sedge Troll with its shield up dies anyway, and the shield is still
/// sitting on it unspent.
#[test]
fn a_regeneration_shield_does_not_save_anything() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let troll = creature(140_000, cards::SEDGE_TROLL, PlayerId::One);
    let troll_id = troll.card.id;
    game.battlefield.push(troll);
    game.players[0].mana_pool.black = 4;
    game.priority = PlayerId::One;
    let regenerate = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == troll_id))
        .expect("the Troll regenerates for a black mana");
    game.apply(PlayerId::One, regenerate)
        .expect("the shield arms");
    pass_priority_pair(&mut game);
    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == troll_id)
            .expect("it is there")
            .regeneration_shields,
        1,
        "the shield is up before the sweeper lands",
    );

    wrath(&mut game);

    assert!(
        !on_battlefield(&game, cards::SEDGE_TROLL),
        "a shield is no answer to a destruction that says it is not",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::SEDGE_TROLL),
        "and it went to the graveyard like anything else",
    );
}

/// "All creatures" is read as it resolves: a Jade Statue that has made
/// itself a Golem is a creature at that moment, and being a land as well is
/// no protection.
#[test]
fn an_animated_artifact_is_swept_with_the_rest() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let statue = game
        .put_onto_battlefield(PlayerId::One, cards::JADE_STATUE)
        .expect("cataloged");
    drain_pending(&mut game);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::BeginningOfCombat;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);
    let animate = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == statue),
        )
        .expect("two mana animates it during combat");
    game.apply(PlayerId::One, animate).expect("it activates");
    drain_pending(&mut game);
    assert!(
        game.permanent_types(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == statue)
                .expect("it is there")
        )
        .is_some_and(|types| types.contains(CardType::Creature)),
        "it is a creature while the sweeper is cast",
    );

    game.step = Step::PrecombatMain;
    wrath(&mut game);

    assert!(
        !on_battlefield(&game, cards::JADE_STATUE),
        "an artifact that made itself a creature is a creature to be destroyed",
    );
}
