//! Flame Slash: the best rate in the format, paid for in timing.

use super::*;

/// Player One holding a Flame Slash with the mana for it, and `theirs` on
/// the battlefield opposite.
fn staged(theirs: CardDefinitionId) -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let victim = creature(79_000, theirs, PlayerId::Two);
    let victim_id = victim.card.id;
    game.battlefield.push(victim);
    let slash = card(79_001, cards::FLAME_SLASH, PlayerId::One);
    let slash_id = slash.id;
    game.players[0].hand.push(slash);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    (game, slash_id, victim_id)
}

fn castable(game: &Game, slash: GameObjectId) -> bool {
    game.legal_actions(PlayerId::One)
        .iter()
        .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == slash))
}

fn permanent(game: &Game, id: GameObjectId) -> Option<&Permanent> {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
}

/// A sorcery, which is the whole price of the rate: their turn is not your
/// turn, and a spell already on the stack closes the window on your own.
#[test]
fn it_waits_for_an_empty_stack_on_your_own_turn() {
    let (mut game, slash, _victim) = staged(cards::SERRA_ANGEL);
    assert!(castable(&game, slash), "your own main phase, stack empty");

    game.active_player = PlayerId::Two;
    assert!(!castable(&game, slash), "and never on theirs");

    game.active_player = PlayerId::One;
    game.stack
        .push(spell(79_100, cards::LIGHTNING_BOLT, PlayerId::Two, 0));
    assert!(
        !castable(&game, slash),
        "nor in response to what they are casting",
    );
}

/// Four damage rather than "destroy": a five-toughness creature lives with
/// the damage marked on it, at the size it always was, and the mark is gone
/// by the next turn.
#[test]
fn four_damage_is_marked_rather_than_lethal() {
    let (mut game, slash, troll) = staged(cards::TROLL_OF_KHAZAD_DUM);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == slash))
        .expect("one red mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    pass_priority_pair(&mut game);
    drain_pending(&mut game);
    game.check_state_based_actions();

    let hurt = permanent(&game, troll).expect("a 6/5 survives four");
    assert_eq!(hurt.damage, 4, "the damage is marked on it");
    assert_eq!(
        (game.power(hurt), game.toughness(hurt)),
        (Some(6), Some(5)),
        "and marked damage is not a smaller creature",
    );

    game.cleanup();
    game.finish_cleanup();

    assert_eq!(
        permanent(&game, troll).expect("still there").damage,
        0,
        "the turn ends and takes the damage with it",
    );
}
