//! Chromatic Star: a card that fixes one mana and replaces itself, and does
//! the second half however it dies rather than only when it is spent.

use super::*;

/// The Star on the battlefield since last turn, with a mana up and a card
/// to draw.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    game.players[0].library.clear();
    game.players[0]
        .library
        .push(card(92_000, cards::LIGHTNING_BOLT, PlayerId::One));
    let star = game
        .put_onto_battlefield(PlayerId::One, cards::CHROMATIC_STAR)
        .expect("cataloged");
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [2, 1];
    drain_pending(&mut game);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    (game, star)
}

fn settle(game: &mut Game) {
    for _ in 0..16 {
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

/// The mana activation that makes `color`, if it is on offer.
fn mana_action(game: &Game, star: GameObjectId, color: ManaColor) -> Option<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateManaAbility {
                source,
                color: made,
                ..
            } => *source == star && *made == color,
            _ => false,
        })
}

fn in_hand(game: &Game, definition: CardDefinitionId) -> bool {
    game.players[0]
        .hand
        .iter()
        .any(|card| card.definition == definition)
}

/// Spending it makes the colour you asked for, and the card follows.
#[test]
fn spending_it_makes_a_color_and_draws() {
    let (mut game, star) = staged();

    let action = mana_action(&game, star, ManaColor::Blue).expect("it makes blue");
    game.apply(PlayerId::One, action).expect("it activates");

    assert_eq!(
        game.players[0].mana_pool.amount(ManaColor::Blue),
        1,
        "the mana is there at once",
    );
    assert!(
        !in_hand(&game, cards::LIGHTNING_BOLT),
        "and the draw is still on the stack",
    );

    settle(&mut game);

    assert!(in_hand(&game, cards::LIGHTNING_BOLT), "now it is drawn");
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::CHROMATIC_STAR),
        "and the Star is in the graveyard",
    );
}

/// Any of the five, and nothing else.
#[test]
fn it_makes_any_of_the_five_colors() {
    let (game, star) = staged();

    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(
            mana_action(&game, star, color).is_some(),
            "{color:?} is on offer",
        );
    }
    assert!(
        mana_action(&game, star, ManaColor::Colorless).is_none(),
        "colourless is not a colour",
    );
}

/// The trigger is on dying rather than on being spent: something else
/// destroying it draws the card just the same, and makes no mana.
#[test]
fn dying_any_other_way_still_draws() {
    let (mut game, star) = staged();

    game.move_permanents_to_graveyard(&[star]);
    settle(&mut game);

    assert!(in_hand(&game, cards::LIGHTNING_BOLT), "the card is drawn");
    assert_eq!(
        game.players[0].mana_pool.total(),
        1,
        "and nothing was added: the mana in the pool is what it started with",
    );
}

/// Without the mana there is nothing to activate.
#[test]
fn it_costs_a_mana_to_spend() {
    let (mut game, star) = staged();
    game.empty_mana_pools();

    assert!(
        mana_action(&game, star, ManaColor::Green).is_none(),
        "the generic mana in the cost has to come from somewhere",
    );
}
