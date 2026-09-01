//! Lion's Eye Diamond: three mana for nothing, once, at the price of
//! everything in hand.

use super::*;

/// The Diamond on the battlefield and `hand` in its controller's hand.
fn staged(hand: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let diamond = game
        .put_onto_battlefield(PlayerId::One, cards::LION_S_EYE_DIAMOND)
        .expect("cataloged");
    for definition in hand {
        let card = game
            .build_zone(PlayerId::One, &[*definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        game.players[0].hand.push(card);
    }
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    drain_pending(&mut game);
    (game, diamond)
}

fn crack(game: &mut Game, diamond: GameObjectId, color: ManaColor) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateManaAbility {
                source,
                color: made,
                ..
            } => *source == diamond && *made == color,
            _ => false,
        })
        .unwrap_or_else(|| panic!("it makes {color:?}"));
    game.apply(PlayerId::One, action).expect("it activates");
}

/// Three of one colour, the hand gone, and the Diamond with it.
#[test]
fn it_trades_your_hand_for_three_mana() {
    let (mut game, diamond) = staged(&[cards::LIGHTNING_BOLT, cards::ISLAND, cards::GRIZZLY_BEARS]);

    crack(&mut game, diamond, ManaColor::Black);

    assert_eq!(game.players[0].mana_pool.black, 3, "three black");
    assert!(game.players[0].hand.is_empty(), "and the hand is gone");
    assert_eq!(
        game.players[0].graveyard.len(),
        4,
        "three cards discarded and the Diamond sacrificed",
    );
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == diamond),
        "the Diamond sacrificed itself",
    );
}

/// Any one colour, and only one: it is three of whichever was chosen.
#[test]
fn it_makes_three_of_any_one_color() {
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        let (mut game, diamond) = staged(&[cards::ISLAND]);

        crack(&mut game, diamond, color);

        assert_eq!(game.players[0].mana_pool.amount(color), 3, "{color:?}");
    }
}

/// An empty hand discards nothing, which is a legal way to pay: the Diamond
/// with nothing to lose is three free mana.
#[test]
fn an_empty_hand_pays_it_too() {
    let (mut game, diamond) = staged(&[]);

    crack(&mut game, diamond, ManaColor::Red);

    assert_eq!(game.players[0].mana_pool.red, 3);
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| { card.definition == cards::LION_S_EYE_DIAMOND })
    );
}

/// "Activate only as an instant": the mana it makes is never reached for
/// while a spell is being paid for, which is what stops it from casting the
/// hand it discards. The Rain costs exactly what the Diamond makes, and the
/// Diamond is the only mana on the board.
#[test]
fn it_never_pays_for_a_spell_being_cast() {
    let (game, _diamond) = staged(&[cards::STONE_RAIN]);
    let rain = game.players[0].hand[0].id;

    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == rain)),
        "the Diamond's three mana cannot pay for the card it would discard",
    );
}

/// The mana outlives the hand: what the deck playing it wants is a use for
/// three mana that the discard itself supplies.
#[test]
fn the_mana_stays_after_the_hand_is_gone() {
    let (mut game, diamond) = staged(&[cards::LIGHTNING_BOLT]);
    let bolt = game.players[0].hand[0].id;

    crack(&mut game, diamond, ManaColor::Red);

    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == bolt)),
        "the Bolt was discarded along with the rest of the hand",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::LIGHTNING_BOLT),
        "and it is in the graveyard",
    );
    assert_eq!(
        game.players[0].mana_pool.red, 3,
        "and the mana is still there"
    );
}

/// Its ruling: "the ability is a mana ability, so it is activated and
/// resolves as a mana ability" -- no stack, no window, and the mana is there
/// the moment it is asked for.
#[test]
fn cracking_it_uses_no_stack() {
    let (mut game, diamond) = staged(&[cards::LIGHTNING_BOLT]);

    crack(&mut game, diamond, ManaColor::Black);

    assert!(
        game.stack.is_empty(),
        "a mana ability resolves where it stands",
    );
    assert_eq!(
        game.players[0].mana_pool.black, 3,
        "with the mana already in the pool",
    );
    assert!(
        game.battlefield.is_empty(),
        "and the Diamond already sacrificed for it",
    );
}

/// "...but it can only be activated at times when you can cast an instant."
/// Their turn is one of those times, which is the whole trick: the hand is
/// spent on their end step and the mana is there for what follows.
#[test]
fn it_may_be_cracked_on_their_turn() {
    let (mut game, diamond) = staged(&[cards::LIGHTNING_BOLT, cards::MOUNTAIN]);
    game.active_player = PlayerId::Two;
    game.step = Step::End;
    game.priority = PlayerId::One;

    crack(&mut game, diamond, ManaColor::Green);

    assert_eq!(
        game.players[0].mana_pool.green, 3,
        "their end step is a time you could cast an instant",
    );
    assert!(
        game.players[0].hand.is_empty(),
        "and the whole hand paid for it",
    );
    assert_eq!(
        game.players[0].graveyard.len(),
        3,
        "two cards discarded and the Diamond that ate them",
    );
}
