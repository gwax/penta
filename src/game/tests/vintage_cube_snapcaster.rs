//! Snapcaster Mage: a 2/1 that hands one card in your graveyard a flashback
//! cost equal to what it already cost.
//!
//! What flashback is, and what it costs, is covered where the mechanism
//! lives. What these check is the card's own reach: which cards it can name,
//! and that the permission it hands out is only a way to pay, not a new time
//! to cast or a cost where there was none.

use super::*;

/// Snapcaster cast with flash on the given player's turn, its trigger
/// pointed at `target` in Player One's graveyard.
fn flash_in(game: &mut Game, active: PlayerId, target: GameObjectId) {
    game.active_player = active;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    let snapcaster = card(90_500, cards::SNAPCASTER_MAGE, PlayerId::One);
    let snapcaster_id = snapcaster.id;
    game.players[PlayerId::One.index()].hand.push(snapcaster);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);

    game.apply(
        PlayerId::One,
        cast_action(snapcaster_id, Vec::new(), Vec::new(), 0),
    )
    .expect("flash makes it castable whenever");
    pass_priority_pair(game);
    let decision = game
        .observe(PlayerId::One)
        .decision
        .expect("the trigger asks which card to name");
    let option = decision
        .options
        .iter()
        .find(|option| option.card.is_some_and(|(object, _)| object == target))
        .map(|option| option.id)
        .expect("the wanted card is on offer");
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![option],
        },
    )
    .expect("naming it is legal");
    pass_priority_pair(game);
    // Casting anything takes priority, which on their turn is theirs to
    // pass first: without this the offers below would be empty for a
    // reason that has nothing to do with flashback.
    game.priority = PlayerId::One;
}

/// Player One with `graveyard` in their graveyard, returned in order.
fn buried(cards: &[CardDefinitionId]) -> (Game, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::One.index()].graveyard.clear();
    let mut ids = Vec::new();
    for (index, definition) in cards.iter().enumerate() {
        let card = card(
            90_000 + u32::try_from(index).expect("a short graveyard"),
            *definition,
            PlayerId::One,
        );
        ids.push(card.id);
        game.players[PlayerId::One.index()].graveyard.push(card);
    }
    game.turns_started = [5, 5];
    (game, ids)
}

/// Whether the card in the graveyard may be cast right now. A spell with
/// targets is offered once per target, so what matters is whether any offer
/// stands rather than how many.
fn can_flash_back(game: &Game, card: GameObjectId) -> bool {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .any(|action| matches!(action, Action::CastSpell { card: cast, .. } if cast == card))
}

/// "You must still follow any timing restrictions and permissions, including
/// those based on the card's type." Snapcaster's own flash is what gets him
/// onto the battlefield on their turn; it does nothing for the sorcery he
/// wakes up, which still waits for a main phase that will not come before
/// the grant expires.
#[test]
fn flashback_is_a_cost_not_a_time_to_cast() {
    let (mut game, ids) = buried(&[cards::WRATH_OF_GOD]);
    flash_in(&mut game, PlayerId::Two, ids[0]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 2);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);

    assert!(
        !can_flash_back(&game, ids[0]),
        "a sorcery is a sorcery, whoever's turn it is",
    );

    // The same grant, on a turn where a sorcery may be cast.
    let (mut game, ids) = buried(&[cards::WRATH_OF_GOD]);
    flash_in(&mut game, PlayerId::One, ids[0]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 2);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);

    assert!(
        can_flash_back(&game, ids[0]),
        "and on your own main phase it is castable",
    );
}

/// An instant is the other half of that: the same grant on their turn is
/// castable at once, which is what Snapcaster is played for.
#[test]
fn an_instant_may_be_flashed_back_on_their_turn() {
    let (mut game, ids) = buried(&[cards::LIGHTNING_BOLT]);
    flash_in(&mut game, PlayerId::Two, ids[0]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    assert!(
        can_flash_back(&game, ids[0]),
        "an instant answers on their turn as readily as on yours",
    );
}

/// "If a card with no mana cost gains flashback, it has no flashback cost.
/// It can't be cast this way." An Ancestral Vision is a sorcery in your
/// graveyard and a legal target, and the grant lands on it and buys nothing.
#[test]
fn a_card_with_no_mana_cost_gains_a_cost_it_cannot_pay() {
    let (mut game, ids) = buried(&[cards::ANCESTRAL_VISION]);
    flash_in(&mut game, PlayerId::One, ids[0]);
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
        ManaColor::Colorless,
    ] {
        game.add_unrestricted_mana(PlayerId::One, color, 5);
    }

    assert!(
        !can_flash_back(&game, ids[0]),
        "a flashback cost equal to no mana cost is no cost to pay",
    );
}

/// "Target instant or sorcery card in your graveyard": a creature of yours
/// is not one, and neither is their instant.
#[test]
fn it_names_your_own_instants_and_sorceries_and_nothing_else() {
    let (mut game, ids) = buried(&[
        cards::LIGHTNING_BOLT,
        cards::GRIZZLY_BEARS,
        cards::WRATH_OF_GOD,
    ]);
    game.players[PlayerId::Two.index()].graveyard.clear();
    let theirs = card(90_400, cards::COUNTERSPELL, PlayerId::Two);
    let theirs_id = theirs.id;
    game.players[PlayerId::Two.index()].graveyard.push(theirs);

    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    let snapcaster = card(90_500, cards::SNAPCASTER_MAGE, PlayerId::One);
    let snapcaster_id = snapcaster.id;
    game.players[PlayerId::One.index()].hand.push(snapcaster);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    game.apply(
        PlayerId::One,
        cast_action(snapcaster_id, Vec::new(), Vec::new(), 0),
    )
    .expect("it is castable");
    pass_priority_pair(&mut game);

    let decision = game
        .observe(PlayerId::One)
        .decision
        .expect("the trigger asks which card to name");
    let offered = decision
        .options
        .iter()
        .filter_map(|option| option.card.map(|(object, _)| object))
        .collect::<Vec<_>>();

    assert!(
        offered.contains(&ids[0]) && offered.contains(&ids[2]),
        "your instant and your sorcery are both on offer: {offered:?}",
    );
    assert!(
        !offered.contains(&ids[1]),
        "a creature card in your graveyard is not one of them",
    );
    assert!(
        !offered.contains(&theirs_id),
        "and their graveyard is not yours to reach into",
    );
}

/// "A spell cast using flashback will always be exiled afterward." The Bolt
/// is cast out of the graveyard, does what it does, and does not come back
/// for a second Snapcaster.
#[test]
fn a_spell_flashed_back_is_exiled_rather_than_buried_again() {
    let (mut game, ids) = buried(&[cards::LIGHTNING_BOLT]);
    let bolt = ids[0];
    flash_in(&mut game, PlayerId::One, bolt);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    let life = game.players[PlayerId::Two.index()].life;

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == bolt
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Player(PlayerId::Two))
            }
            _ => false,
        })
        .expect("one red pays the flashback cost of a Bolt");
    game.apply(PlayerId::One, cast).expect("it is cast");
    pass_priority_pair(&mut game);
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert_eq!(
        game.players[PlayerId::Two.index()].life,
        life - 3,
        "it resolved out of the graveyard",
    );
    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .all(|card| card.definition != cards::LIGHTNING_BOLT),
        "and it did not go back where it came from",
    );
    assert!(
        game.players[PlayerId::One.index()]
            .exile
            .iter()
            .any(|card| card.definition == cards::LIGHTNING_BOLT),
        "flashback exiles what it casts",
    );
}

/// "The flashback cost is equal to its mana cost": no discount and no
/// surcharge, so a Counterspell wants its two blue and is not offered for
/// one.
#[test]
fn the_flashback_cost_is_the_printed_mana_cost() {
    let (mut game, ids) = buried(&[cards::FRANTIC_SEARCH]);
    let search = ids[0];
    flash_in(&mut game, PlayerId::Two, search);

    // The Search is {2}{U}, and its flashback cost is the same three mana:
    // no discount for coming back, and no surcharge either.
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    assert!(
        !can_flash_back(&game, search),
        "two mana does not pay a three-mana cost",
    );

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    assert!(
        can_flash_back(&game, search),
        "and the third is the whole of what is left to pay",
    );
}

/// The grant is "until end of turn", so a card left in the graveyard is an
/// ordinary card there again on the next turn.
#[test]
fn the_grant_is_gone_the_following_turn() {
    let (mut game, ids) = buried(&[cards::LIGHTNING_BOLT]);
    let bolt = ids[0];
    flash_in(&mut game, PlayerId::One, bolt);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    assert!(can_flash_back(&game, bolt), "castable while the turn lasts");

    // The grant expires in the cleanup step, which is where an
    // until-end-of-turn effect goes away rather than at the turn boundary.
    game.cleanup();
    game.start_next_turn();
    drain_pending(&mut game);
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);

    assert!(
        !can_flash_back(&game, bolt),
        "and an ordinary graveyard card once the turn is over",
    );
    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::LIGHTNING_BOLT),
        "still lying where it was",
    );
}
