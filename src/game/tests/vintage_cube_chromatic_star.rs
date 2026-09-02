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

/// The trigger reads "put into a graveyard from the battlefield": a Star
/// exiled instead never reaches one, so the card stays in the library.
#[test]
fn exiling_it_draws_nothing() {
    let (mut game, star) = staged();
    let library = game.players[PlayerId::One.index()].library.len();

    game.exile_permanent(star);
    settle(&mut game);

    assert!(
        game.players[PlayerId::One.index()]
            .exile
            .iter()
            .any(|card| card.definition == cards::CHROMATIC_STAR),
        "the Star is in exile",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].library.len(),
        library,
        "and nothing was drawn on the way",
    );
    assert!(
        game.players[PlayerId::One.index()].graveyard.is_empty(),
        "it never touched a graveyard",
    );
}

/// The sacrifice is a cost, so the Star answers the removal pointed at it:
/// spent in response to an Abrade, it makes its mana, draws its card, and
/// leaves the spell with nothing to destroy.
#[test]
fn spending_it_in_response_leaves_their_removal_with_nothing() {
    let (mut game, star) = staged();
    let abrade = card(92_500, cards::ABRADE, PlayerId::Two);
    let abrade_id = abrade.id;
    game.players[PlayerId::Two.index()].hand.push(abrade);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Red, 1);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Colorless, 1);
    game.priority = PlayerId::Two;

    let cast = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == abrade_id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(star))
            }
            _ => false,
        })
        .expect("Abrade's second mode names an artifact");
    game.apply(PlayerId::Two, cast).expect("it is cast");

    game.priority = PlayerId::One;
    let green = mana_action(&game, star, ManaColor::Green)
        .expect("the Star may be spent while their spell waits");
    game.apply(PlayerId::One, green).expect("it activates");
    settle(&mut game);

    assert_eq!(
        game.players[PlayerId::One.index()].mana_pool.green,
        1,
        "the mana was made before their spell resolved",
    );
    assert!(
        in_hand(&game, cards::LIGHTNING_BOLT),
        "and the card came with it",
    );
    assert!(
        game.players[PlayerId::Two.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::ABRADE),
        "their Abrade resolved into nothing and is spent",
    );
}

/// The two halves keep their own timing: the mana ability uses no stack, so
/// the colour is in the pool the moment it is announced, while the draw is
/// an ordinary trigger that waits its turn.
#[test]
fn the_mana_is_there_before_the_card_is() {
    let (mut game, star) = staged();

    let action = mana_action(&game, star, ManaColor::Blue).expect("blue is on offer");
    game.apply(PlayerId::One, action).expect("it activates");

    assert_eq!(
        game.players[0].mana_pool.blue, 1,
        "the mana is already spendable",
    );
    assert!(
        !in_hand(&game, cards::LIGHTNING_BOLT),
        "and the card it owes is still waiting on the stack",
    );
    assert!(
        !game.stack.is_empty() || !game.pending_triggers.is_empty(),
        "which is where the draw is",
    );

    settle(&mut game);

    assert!(in_hand(&game, cards::LIGHTNING_BOLT), "and then it arrives");
}

/// "Put into a graveyard *from the battlefield*": a Star returned to hand
/// never reaches one, so it draws nothing -- the other half of the boundary
/// the exile case draws.
#[test]
fn bouncing_it_to_hand_draws_nothing() {
    let (mut game, star) = staged();

    game.return_permanent_to_hand(star);
    drain_pending(&mut game);
    settle(&mut game);

    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::CHROMATIC_STAR),
        "the Star is in hand",
    );
    assert!(
        !in_hand(&game, cards::LIGHTNING_BOLT),
        "and its card stayed in the library",
    );
    assert_eq!(game.players[0].library.len(), 1, "untouched");
}

/// The line the card is played for, start to finish: an artifact has no
/// summoning sickness, so the Star may be cast and cracked on the same turn,
/// and the colour it makes is the pip the spell in your hand was missing.
/// The card it draws comes after the spell it paid for is already cast.
#[test]
fn it_is_cast_cracked_and_spent_on_one_turn() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    game.players[0].library.clear();
    game.players[0]
        .library
        .push(card(92_402, cards::GRIZZLY_BEARS, PlayerId::One));
    game.turns_started = [2, 1];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    let star = card(92_400, cards::CHROMATIC_STAR, PlayerId::One);
    let star_id = star.id;
    game.players[0].hand.push(star);
    let bolt = card(92_401, cards::LIGHTNING_BOLT, PlayerId::One);
    let bolt_id = bolt.id;
    game.players[0].hand.push(bolt);
    // One colourless for the Star, one for its ability, and no red at all.
    game.players[0].mana_pool = ManaPool::default();
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);

    let cast_star = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == star_id))
        .expect("one mana casts it");
    game.apply(PlayerId::One, cast_star).expect("it is cast");
    settle(&mut game);
    let star = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::CHROMATIC_STAR)
        .expect("it arrived")
        .card
        .id;
    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == bolt_id)),
        "with no red up the Bolt is not castable yet",
    );

    let crack = mana_action(&game, star, ManaColor::Red).expect("the Star makes red on arrival");
    game.apply(PlayerId::One, crack).expect("it is spent");

    let cast_bolt = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == bolt_id))
        .expect("and the red it made pays for the Bolt");
    game.apply(PlayerId::One, cast_bolt).expect("it is cast");
    settle(&mut game);

    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::CHROMATIC_STAR),
        "the Star was the cost and is in the graveyard",
    );
    assert!(
        in_hand(&game, cards::GRIZZLY_BEARS),
        "and the card it drew arrived after the spell it paid for",
    );
}
