//! Vampiric Tutor: any card in the deck, on top, for one mana and two life.
//!
//! That the card survives the shuffle and that the two life is paid is
//! pinned in `vintage_cube_library`, beside the Imperial Seal that prints
//! the same clause. What is here is the half that separates the two life
//! from a cost: it is part of the resolution, and it is happy to kill you.

use super::*;

/// Player One at `life` with a Tutor in hand, one black up, and three cards
/// in the library.
fn staged(life: i16) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for (index, definition) in [
        cards::GRIZZLY_BEARS,
        cards::SERRA_ANGEL,
        cards::LIGHTNING_BOLT,
    ]
    .into_iter()
    .enumerate()
    {
        game.players[0].library.push(card(
            121_000 + u32::try_from(index).expect("three cards"),
            definition,
            PlayerId::One,
        ));
    }
    let tutor = game
        .build_zone(PlayerId::One, &[cards::VAMPIRIC_TUTOR])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = tutor.id;
    game.players[0].hand.push(tutor);
    game.players[0].life = life;
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    (game, id)
}

/// Casts the Tutor and takes the Angel out of the library.
fn tutor_for_the_angel(game: &mut Game, tutor: GameObjectId) {
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == tutor))
        .expect("one black casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    pass_priority_pair(game);

    let decision = game
        .observe(PlayerId::One)
        .decision
        .expect("the search asks what to find");
    let angel = decision
        .options
        .iter()
        .find(|option| {
            option.card.is_some_and(|(_, characteristics)| {
                characteristics.card_definition() == Some(cards::SERRA_ANGEL)
            })
        })
        .expect("every card in the library is eligible")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![angel],
        },
    )
    .expect("the search is answered");
    drain_pending(game);
}

/// "You lose 2 life" is not a cost: at two life the Tutor is castable, the
/// card goes on top, and the loss takes the game with it.
#[test]
fn it_will_take_your_last_two_life() {
    let (mut game, tutor) = staged(2);

    tutor_for_the_angel(&mut game, tutor);
    game.check_state_based_actions();

    assert_eq!(
        game.players[0].library.last().map(|card| card.definition),
        Some(cards::SERRA_ANGEL),
        "the card it found is on top all the same",
    );
    assert_eq!(game.players[0].life, 0, "and the two came off");
    assert_eq!(
        game.result,
        Some(GameResult::Winner {
            winner: PlayerId::Two,
            reason: WinReason::OpponentLostAllLife,
        }),
        "which is a loss rather than an unpayable cost",
    );
}

/// One life is enough to cast it, which is what "not a cost" means: nothing
/// checks the total before the spell resolves.
#[test]
fn one_life_is_no_obstacle_to_casting_it() {
    let (game, tutor) = staged(1);

    assert!(
        game.legal_actions(PlayerId::One)
            .into_iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if card == tutor)),
        "it is castable at one life, unlike a spell that asks for two as a cost",
    );
}
