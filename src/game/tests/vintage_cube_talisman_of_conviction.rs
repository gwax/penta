//! Talisman of Conviction: colorless for nothing, or a colour for a life.

use super::*;

fn staged() -> (Game, GameObjectId) {
    staged_with(cards::TALISMAN_OF_CONVICTION)
}

/// The same, for any Talisman in the cycle: they differ only in their pair.
fn staged_with(talisman: CardDefinitionId) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let talisman = game
        .put_onto_battlefield(PlayerId::One, talisman)
        .expect("cataloged");
    drain_pending(&mut game);
    game.players[0].life = 20;
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, talisman)
}

fn mana_action(game: &Game, source: GameObjectId, color: ManaColor) -> Option<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateManaAbility {
                source: activated,
                color: made,
                ..
            } => *activated == source && *made == color,
            _ => false,
        })
}

/// Colorless costs nothing.
#[test]
fn colorless_is_free() {
    let (mut game, talisman) = staged();

    let colorless = mana_action(&game, talisman, ManaColor::Colorless).expect("it makes {C}");
    game.apply(PlayerId::One, colorless).expect("it activates");

    assert_eq!(game.players[0].mana.len(), 1);
    assert_eq!(game.players[0].life, 20, "and takes nothing for it");
}

/// Either colour costs a life, and the damage comes from the artifact.
#[test]
fn a_color_costs_a_life() {
    for color in [ManaColor::Red, ManaColor::White] {
        let (mut game, talisman) = staged();

        let colored = mana_action(&game, talisman, color).expect("it makes both colours");
        game.apply(PlayerId::One, colored).expect("it activates");
        game.check_state_based_actions();

        assert_eq!(game.players[0].mana.len(), 1);
        assert_eq!(game.players[0].life, 19, "one damage from the artifact");
    }
}

/// It makes no colour it does not print.
#[test]
fn it_makes_only_its_two_colors() {
    let (game, talisman) = staged();

    for color in [ManaColor::Blue, ManaColor::Black, ManaColor::Green] {
        assert!(
            mana_action(&game, talisman, color).is_none(),
            "{color:?} is not one of its colours",
        );
    }
}

/// Tapping for one closes the other: the tap is the cost of both.
#[test]
fn the_tap_pays_for_only_one_of_them() {
    let (mut game, talisman) = staged();

    let colorless = mana_action(&game, talisman, ManaColor::Colorless).expect("it makes {C}");
    game.apply(PlayerId::One, colorless).expect("it activates");

    assert!(mana_action(&game, talisman, ManaColor::Red).is_none());
    assert!(mana_action(&game, talisman, ManaColor::Colorless).is_none());
}

/// The blue-red half of the cycle, which is the same card with its own two
/// colours.
#[test]
fn the_blue_red_talisman_makes_its_own_two() {
    let (mut game, talisman) = staged_with(cards::TALISMAN_OF_CREATIVITY);

    for color in [ManaColor::White, ManaColor::Black, ManaColor::Green] {
        assert!(
            mana_action(&game, talisman, color).is_none(),
            "{color:?} is not one of its colours",
        );
    }

    let blue = mana_action(&game, talisman, ManaColor::Blue).expect("it makes {U}");
    game.apply(PlayerId::One, blue).expect("it activates");
    game.check_state_based_actions();

    assert_eq!(game.players[0].mana.len(), 1);
    assert_eq!(game.players[0].life, 19, "one damage from the artifact");
}

/// The damage is what the ability does rather than what it costs, so no
/// amount of life stands between you and it: at one life the coloured half
/// is still on offer, and taking it is a loss. (Contrast a Phyrexian pip,
/// where the life is a cost and CR 118.4 refuses it.)
#[test]
fn one_life_does_not_stop_the_colored_half() {
    let (mut game, talisman) = staged();
    game.players[0].life = 1;

    let red = mana_action(&game, talisman, ManaColor::Red).expect("the offer does not ask");
    game.apply(PlayerId::One, red).expect("it activates");
    game.check_state_based_actions();

    assert_eq!(game.players[0].mana.len(), 1, "the mana was made");
    assert_eq!(game.players[0].life, 0);
    assert_eq!(
        game.result(),
        Some(GameResult::Winner {
            winner: PlayerId::Two,
            reason: WinReason::OpponentLostAllLife,
        }),
        "and the artifact killed its own controller",
    );
}
