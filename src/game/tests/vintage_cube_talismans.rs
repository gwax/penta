//! The Talisman cycle the cube plays: colourless for nothing, or one of two
//! colours for a life. The five differ only in the pair they print, so what
//! is worth checking is checked across all of them at once.

use super::*;

/// The five Talismans the cube plays, with the pair each one prints.
const CUBE_TALISMANS: [(CardDefinitionId, [ManaColor; 2]); 5] = [
    (
        cards::TALISMAN_OF_CONVICTION,
        [ManaColor::Red, ManaColor::White],
    ),
    (
        cards::TALISMAN_OF_CREATIVITY,
        [ManaColor::Blue, ManaColor::Red],
    ),
    (
        cards::TALISMAN_OF_CURIOSITY,
        [ManaColor::Green, ManaColor::Blue],
    ),
    (
        cards::TALISMAN_OF_DOMINANCE,
        [ManaColor::Blue, ManaColor::Black],
    ),
    (
        cards::TALISMAN_OF_PROGRESS,
        [ManaColor::White, ManaColor::Blue],
    ),
];

const EVERY_COLOR: [ManaColor; 5] = [
    ManaColor::White,
    ManaColor::Blue,
    ManaColor::Black,
    ManaColor::Red,
    ManaColor::Green,
];

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

/// Colourless costs nothing.
#[test]
fn colorless_is_free() {
    for (definition, _) in CUBE_TALISMANS {
        let (mut game, talisman) = staged_with(definition);

        let colorless = mana_action(&game, talisman, ManaColor::Colorless)
            .unwrap_or_else(|| panic!("{definition:?} makes colorless"));
        game.apply(PlayerId::One, colorless).expect("it activates");
        game.check_state_based_actions();

        assert_eq!(game.players[0].mana.len(), 1, "{definition:?}");
        assert_eq!(
            game.players[0].life, 20,
            "{definition:?} takes nothing for it",
        );
    }
}

/// Each Talisman makes its own two colours for a life each, and no colour it
/// does not print.
#[test]
fn each_one_makes_its_own_two_colors_and_no_others() {
    for (definition, colors) in CUBE_TALISMANS {
        for color in EVERY_COLOR {
            let (mut game, talisman) = staged_with(definition);
            let Some(action) = mana_action(&game, talisman, color) else {
                assert!(
                    !colors.contains(&color),
                    "{definition:?} prints {color:?} and would not make it",
                );
                continue;
            };
            assert!(
                colors.contains(&color),
                "{definition:?} does not print {color:?} and made it anyway",
            );

            game.apply(PlayerId::One, action).expect("it activates");
            game.check_state_based_actions();

            assert_eq!(game.players[0].mana.len(), 1, "{definition:?} {color:?}");
            assert_eq!(
                game.players[0].life, 19,
                "{definition:?} takes a life for {color:?}",
            );
        }
    }
}

/// Tapping for one closes the others: the tap is the cost of all three
/// abilities.
#[test]
fn the_tap_pays_for_only_one_of_them() {
    for (definition, colors) in CUBE_TALISMANS {
        let (mut game, talisman) = staged_with(definition);

        let colorless = mana_action(&game, talisman, ManaColor::Colorless)
            .unwrap_or_else(|| panic!("{definition:?} makes colorless"));
        game.apply(PlayerId::One, colorless).expect("it activates");

        assert!(
            mana_action(&game, talisman, ManaColor::Colorless).is_none(),
            "{definition:?} is tapped",
        );
        for color in colors {
            assert!(
                mana_action(&game, talisman, color).is_none(),
                "{definition:?} cannot then make {color:?}",
            );
        }
    }
}

/// The card prints damage, not a life payment. Anything watching for damage
/// sees it, and nothing watching for life loss does.
#[test]
fn the_life_it_costs_is_dealt_as_damage() {
    for (definition, colors) in CUBE_TALISMANS {
        let (mut game, talisman) = staged_with(definition);
        let before = game.events.len();

        let action = mana_action(&game, talisman, colors[0])
            .unwrap_or_else(|| panic!("{definition:?} makes {:?}", colors[0]));
        game.apply(PlayerId::One, action).expect("it activates");
        game.check_state_based_actions();

        let events = &game.events[before..];
        assert!(
            events.iter().any(|event| matches!(
                event,
                GameEvent::DamageDealt {
                    player: PlayerId::One,
                    amount: 1,
                }
            )),
            "{definition:?} dealt one damage: {events:?}",
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, GameEvent::LifeLost { .. })),
            "{definition:?} paid nothing",
        );
    }
}

/// The damage is what the ability does rather than what it costs, so no
/// amount of life stands between you and it: at one life the coloured half
/// is still on offer, and taking it is a loss. (Contrast a Phyrexian pip,
/// where the life is a cost and CR 118.4 refuses it.)
#[test]
fn one_life_does_not_stop_the_colored_half() {
    let (mut game, talisman) = staged_with(cards::TALISMAN_OF_CONVICTION);
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
