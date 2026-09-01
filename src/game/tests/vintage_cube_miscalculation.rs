//! Miscalculation against a spell that cannot be countered.
//!
//! The tax itself -- how much it asks, what paying and declining do, and
//! that it names creature spells where a Spell Pierce cannot -- is pinned
//! across the soft-counter trio in `vintage_cube_spells`, and its cycling is
//! in `premodern_cycling`. What is here is the case neither covers: the
//! "unless" is still offered against a spell the counter half can do nothing
//! to, so the choice is real and the outcome is not.

use super::*;

/// Player Two casting a Supreme Verdict into Player One's Miscalculation,
/// stopped at the payment decision.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[1].hand.clear();
    game.battlefield
        .push(creature(126_000, cards::GRIZZLY_BEARS, PlayerId::One));
    let verdict = card(126_001, cards::SUPREME_VERDICT, PlayerId::Two);
    let verdict_id = verdict.id;
    game.players[1].hand.push(verdict);
    let miscalculation = card(126_002, cards::MISCALCULATION, PlayerId::One);
    let miscalculation_id = miscalculation.id;
    game.players[0].hand.push(miscalculation);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::Two;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::Two;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 2);
    for color in ManaColor::COLORS {
        game.add_unrestricted_mana(PlayerId::Two, color, 4);
    }

    game.apply(
        PlayerId::Two,
        cast_action(verdict_id, Vec::new(), Vec::new(), 0),
    )
    .expect("four mana casts the Verdict");
    let on_stack = game.stack.last().expect("it is on the stack").id;
    game.apply(PlayerId::Two, Action::PassPriority)
        .expect("they pass");
    game.apply(
        PlayerId::One,
        cast_action(
            miscalculation_id,
            vec![Target::Spell(on_stack)],
            Vec::new(),
            0,
        ),
    )
    .expect("two mana answers it");
    pass_priority_pair(&mut game);
    (game, miscalculation_id)
}

/// Answers the tax, paying it when `pay`.
fn answer(game: &mut Game, pay: bool) {
    let decision = game
        .observe(PlayerId::Two)
        .decision
        .expect("its controller is asked to pay");
    let wanted = if pay { "Pay the cost" } else { "Decline" };
    let option = decision
        .options
        .iter()
        .find(|option| option.label == wanted)
        .unwrap_or_else(|| panic!("{wanted} is one of the two: {:?}", decision.options))
        .id;
    game.apply(
        PlayerId::Two,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![option],
        },
    )
    .expect("the answer is legal");
    drain_pending(game);
}

/// The tax is asked even of a spell that cannot be countered, and declining
/// it counters nothing: the Verdict resolves and sweeps the board.
#[test]
fn the_tax_is_offered_and_buys_nothing_against_an_uncounterable_spell() {
    let (mut game, _miscalculation) = staged();
    let mana = game.players[1].mana.len();

    answer(&mut game, false);

    assert!(
        game.battlefield.is_empty(),
        "the Verdict resolved and swept the board",
    );
    assert!(
        game.players[1]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::SUPREME_VERDICT),
        "and went where a resolved sorcery goes",
    );
    assert_eq!(
        game.players[1].mana.len(),
        mana,
        "declining spent nothing on it",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::MISCALCULATION),
        "while the Miscalculation is spent either way",
    );
}

/// Paying it is worse than declining: the two mana is taken and the spell
/// was never going to be countered.
#[test]
fn paying_the_tax_against_it_is_two_mana_thrown_away() {
    let (mut game, _miscalculation) = staged();
    let mana = game.players[1].mana.len();

    answer(&mut game, true);

    assert_eq!(
        game.players[1].mana.len(),
        mana - 2,
        "the two was taken from them",
    );
    assert!(
        game.battlefield.is_empty(),
        "and the Verdict resolved, as it would have anyway",
    );
}
