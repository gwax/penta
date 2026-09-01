//! Nissa, Who Shakes the World: doubled Forests, an awakened land, and the
//! ultimate that protects what it fetches.

use super::*;

/// Resolves whatever is on the stack, answering nothing.
fn resolve(game: &mut Game) {
    for _ in 0..12 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
}

/// Puts Nissa on the battlefield with the loyalty she would have entered
/// carrying, and returns her object id.
///
/// A permanent built by hand has no counters, and a planeswalker at zero
/// loyalty is binned by the first state-based check -- which an entering
/// land carrying an ability of its own is enough to run. Without the
/// counters she survives only until something makes the game look, and a
/// clause of hers read after that quietly answers nothing.
fn nissa_on_the_battlefield(game: &mut Game, id: u32) -> GameObjectId {
    let mut nissa = creature(id, cards::NISSA_WHO_SHAKES_THE_WORLD, PlayerId::One);
    nissa.set_counters(CounterKind::Loyalty, 5);
    let nissa_id = nissa.card.id;
    game.battlefield.push(nissa);
    nissa_id
}

/// Activates Nissa's +1 at `land` and lets it resolve.
fn animate(game: &mut Game, nissa: GameObjectId, land: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } => {
                *source == nissa
                    && targets
                        .iter()
                        .flat_map(crate::casting::TargetSelection::targets)
                        .any(|target| *target == Target::Permanent(land))
            }
            _ => false,
        })
        .expect("the +1 can point at a noncreature land");
    game.apply(PlayerId::One, action).expect("it is activated");
    resolve(game);
    drain_pending(game);
}

/// Tapping a Forest for mana adds a second green, and tapping something else
/// does not.
#[test]
fn nissa_doubles_your_forests_and_nothing_else() {
    for (land, expected) in [(cards::FOREST, 2), (cards::MOUNTAIN, 0)] {
        let mut game = ready_game();
        game.battlefield.clear();
        nissa_on_the_battlefield(&mut game, 85_000);
        let source = creature(85_001, land, PlayerId::One);
        let source_id = source.card.id;
        game.battlefield.push(source);

        let color = if land == cards::FOREST {
            ManaColor::Green
        } else {
            ManaColor::Red
        };
        game.apply(
            PlayerId::One,
            Action::ActivateManaAbility {
                source: source_id,
                ability: mana_ability_for(&game, source_id, color),
                color,
                counters_removed: None,
                cost_object: None,
                combination: None,
                triggered_mana: None,
            },
        )
        .expect("the land taps for mana");
        drain_pending(&mut game);

        assert_eq!(
            game.players[0].mana_pool.green, expected,
            "{land:?} should make {expected} green",
        );
    }
}

/// The +1 grows a land into a 3/3 that is still a land, and that can attack
/// the turn it wakes up.
#[test]
fn the_plus_one_wakes_a_land_that_is_still_a_land() {
    let mut game = ready_game();
    game.battlefield.clear();
    let nissa_id = nissa_on_the_battlefield(&mut game, 85_010);
    let forest = creature(85_011, cards::FOREST, PlayerId::One);
    let forest_id = forest.card.id;
    game.battlefield.push(forest);

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } => {
                *source == nissa_id
                    && targets
                        .iter()
                        .flat_map(crate::casting::TargetSelection::targets)
                        .any(|target| *target == Target::Permanent(forest_id))
            }
            _ => false,
        })
        .expect("the +1 can point at a noncreature land");
    game.apply(PlayerId::One, action).expect("it is activated");
    resolve(&mut game);
    drain_pending(&mut game);

    let land = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == forest_id)
        .expect("the land is still there");
    assert_eq!(game.power(land), Some(3), "three counters on a 0/0 base");
    assert_eq!(game.toughness(land), Some(3));
    assert!(
        game.permanent_has_executable_keyword(land, KeywordAbility::Haste),
        "and it can attack at once",
    );
    assert!(
        game.permanent_has_executable_keyword(land, KeywordAbility::Vigilance),
        "without tapping to do it",
    );
    let types = game
        .permanent_types(land)
        .expect("an animated land has types");
    assert!(
        types.contains(CardType::Land),
        "still a land, which is why Nissa still doubles it",
    );
    assert!(types.is_creature());
}

/// The ultimate fetches every Forest in the library and leaves an emblem
/// that keeps them alive.
#[test]
fn the_ultimate_fetches_the_forests_and_protects_them() {
    let mut game = ready_game();
    game.battlefield.clear();
    let mut nissa = creature(85_020, cards::NISSA_WHO_SHAKES_THE_WORLD, PlayerId::One);
    nissa.add_counters(CounterKind::Loyalty, 8);
    let nissa_id = nissa.card.id;
    game.battlefield.push(nissa);
    game.players[0].library.clear();
    for id in 85_021..85_024 {
        game.players[0]
            .library
            .push(card(id, cards::FOREST, PlayerId::One));
    }
    game.players[0]
        .library
        .push(card(85_030, cards::MOUNTAIN, PlayerId::One));

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { source, .. } if *source == nissa_id)
                && matches!(action, Action::ActivateAbility { ability, .. }
                    if game
                        .ability_for_origin(nissa_id, *ability)
                        .is_some_and(|ability| ability.text.starts_with('\u{2212}')))
        })
        .expect("eight loyalty pays for the ultimate");
    game.apply(PlayerId::One, action).expect("it is activated");
    resolve(&mut game);

    // "Any number" is sized from the library, so the search offers every
    // Forest in it and none of the Mountain.
    let search = game
        .pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("the search asks which cards to take");
    assert_eq!((search.minimum, search.maximum), (0, 3));
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: search.id,
            options: search.options.iter().map(|option| option.id).collect(),
        },
    )
    .expect("taking all three is a legal answer");
    drain_pending(&mut game);

    let forests = game
        .battlefield
        .iter()
        .filter(|permanent| permanent.card.definition == cards::FOREST)
        .count();
    assert_eq!(forests, 3, "every Forest in the library arrives");
    assert!(
        game.battlefield
            .iter()
            .filter(|permanent| permanent.card.definition == cards::FOREST)
            .all(|permanent| permanent.tapped),
        "tapped, as the card says",
    );
    assert_eq!(game.emblems.len(), 1, "and the emblem is made");
}

/// "The effect of Nissa's first loyalty ability lasts indefinitely. It
/// doesn't wear off during the cleanup step." Most animation is until end of
/// turn; this one is not, so the land is still a 3/3 Elemental a turn later.
#[test]
fn the_awakened_land_is_still_a_creature_next_turn() {
    let mut game = ready_game();
    game.battlefield.clear();
    let nissa_id = nissa_on_the_battlefield(&mut game, 85_040);
    let forest = creature(85_041, cards::FOREST, PlayerId::One);
    let forest_id = forest.card.id;
    game.battlefield.push(forest);
    animate(&mut game, nissa_id, forest_id);

    // Walking the steps rather than jumping the turn: the cleanup step is
    // where an ordinary animation would be let go of.
    let turn = game.turn;
    for _ in 0..60 {
        if game.turn > turn {
            break;
        }
        game.advance_step();
        drain_pending(&mut game);
    }
    assert!(game.turn > turn, "a whole turn has passed");

    let land = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == forest_id)
        .expect("the land is still there");
    assert_eq!(
        (game.power(land), game.toughness(land)),
        (Some(3), Some(3)),
        "the counters and the 0/0 base both survived the cleanup",
    );
    assert!(
        game.permanent_types(land)
            .is_some_and(super::CardTypeSet::is_creature),
        "and it is still an Elemental",
    );
}

/// "Still a land": an awakened Forest keeps its mana ability, and Nissa's
/// own static reads it as the Forest it never stopped being.
#[test]
fn an_awakened_forest_still_taps_for_two_green() {
    let mut game = ready_game();
    game.battlefield.clear();
    let nissa_id = nissa_on_the_battlefield(&mut game, 85_050);
    let forest = creature(85_051, cards::FOREST, PlayerId::One);
    let forest_id = forest.card.id;
    game.battlefield.push(forest);
    animate(&mut game, nissa_id, forest_id);

    game.apply(
        PlayerId::One,
        Action::ActivateManaAbility {
            source: forest_id,
            ability: mana_ability_for(&game, forest_id, ManaColor::Green),
            color: ManaColor::Green,
            counters_removed: None,
            cost_object: None,
            combination: None,
            triggered_mana: None,
        },
    )
    .expect("an animated land is still a land to tap");
    resolve(&mut game);
    drain_pending(&mut game);

    assert_eq!(
        game.players[0].mana_pool.green, 2,
        "one for the Forest and one for Nissa",
    );
}

/// "Target noncreature land you control": a land she has already woken is a
/// creature, so she cannot point at it twice. A second Forest beside it is
/// what shows the +1 is on offer at all.
#[test]
fn she_cannot_wake_the_same_land_twice() {
    let mut game = ready_game();
    game.battlefield.clear();
    let nissa_id = nissa_on_the_battlefield(&mut game, 85_060);
    let forest = creature(85_061, cards::FOREST, PlayerId::One);
    let forest_id = forest.card.id;
    game.battlefield.push(forest);
    let spare = creature(85_062, cards::FOREST, PlayerId::One);
    let spare_id = spare.card.id;
    game.battlefield.push(spare);
    animate(&mut game, nissa_id, forest_id);

    // A loyalty ability is once a turn, so the next turn is where asking
    // again means anything.
    let turn = game.turn;
    for _ in 0..60 {
        if game.turn > turn {
            break;
        }
        game.advance_step();
        drain_pending(&mut game);
    }
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    let targets = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } if source == nissa_id => Some(
                targets
                    .iter()
                    .flat_map(crate::casting::TargetSelection::targets)
                    .copied()
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .flatten()
        .collect::<Vec<_>>();

    assert!(
        targets.contains(&Target::Permanent(spare_id)),
        "the untouched Forest is still a noncreature land: {targets:?}",
    );
    assert!(
        !targets.contains(&Target::Permanent(forest_id)),
        "and what she woke is a creature land the +1 cannot name again",
    );
}

/// Taps `land` for `color` under `player`.
fn tap_for(game: &mut Game, land: GameObjectId, color: ManaColor, player: PlayerId) {
    game.priority = player;
    game.apply(
        player,
        Action::ActivateManaAbility {
            source: land,
            ability: mana_ability_for(game, land, color),
            color,
            counters_removed: None,
            cost_object: None,
            combination: None,
            triggered_mana: None,
        },
    )
    .expect("the land taps for mana");
    drain_pending(game);
}

/// "Whenever you tap a Forest *for mana*" -- it does not say for green. A
/// Stomping Ground tapped for red is still a Forest being tapped, so the
/// extra green comes anyway and the pool holds one of each.
#[test]
fn a_forest_tapped_for_another_colour_still_adds_green() {
    let mut game = ready_game();
    game.battlefield.clear();
    nissa_on_the_battlefield(&mut game, 85_110);
    let ground = land_ready(&mut game, cards::STOMPING_GROUND);

    tap_for(&mut game, ground, ManaColor::Red, PlayerId::One);

    assert_eq!(
        game.players[0].mana_pool.red, 1,
        "the red it was tapped for"
    );
    assert_eq!(
        game.players[0].mana_pool.green, 1,
        "and the green her clause adds for the tapping itself",
    );
}

/// "Whenever *you* tap a Forest": theirs is their own business, and nobody
/// is handed a second green for it.
#[test]
fn their_forest_adds_nothing_to_anyone() {
    let mut game = ready_game();
    game.battlefield.clear();
    nissa_on_the_battlefield(&mut game, 85_120);
    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::FOREST)
        .expect("cataloged");
    drain_pending(&mut game);

    tap_for(&mut game, theirs, ManaColor::Green, PlayerId::Two);

    assert_eq!(
        game.players[1].mana_pool.green, 1,
        "one green, and no more, for the seat that tapped it",
    );
    assert_eq!(
        game.players[0].mana_pool.green, 0,
        "and nothing at all for Nissa's own",
    );
}

/// "Up to one target noncreature land": none is a legal answer, so the +1 is
/// on offer with no land to point it at and simply grows her loyalty.
#[test]
fn the_plus_one_may_name_nothing() {
    let mut game = ready_game();
    game.battlefield.clear();
    let nissa = nissa_on_the_battlefield(&mut game, 85_130);
    let before = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == nissa)
        .expect("she is out")
        .counters(CounterKind::Loyalty);

    let empty = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } => {
                *source == nissa
                    && targets
                        .iter()
                        .all(|selection| selection.targets().is_empty())
            }
            _ => false,
        })
        .expect("naming nothing is a legal way to use it");
    game.apply(PlayerId::One, empty).expect("it activates");
    resolve(&mut game);

    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == nissa)
            .expect("she is still there")
            .counters(CounterKind::Loyalty),
        before + 1,
        "the loyalty went up with nothing to show for it",
    );
}

/// Puts `definition` down untapped, whatever its own entry clause wanted.
fn land_ready(game: &mut Game, definition: CardDefinitionId) -> GameObjectId {
    let land = game
        .put_onto_battlefield(PlayerId::One, definition)
        .expect("cataloged");
    drain_pending(game);
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == land)
    {
        permanent.tapped = false;
    }
    land
}

/// "A Forest" is the land type rather than the card name, and the cube's
/// Forests are mostly nonbasic: an ability-free Taiga, two shocklands and a
/// surveil land all carry the type, and every one of them doubles.
#[test]
fn every_land_carrying_the_forest_type_doubles() {
    for definition in [
        cards::TAIGA,
        cards::STOMPING_GROUND,
        cards::TEMPLE_GARDEN,
        cards::COMMERCIAL_DISTRICT,
    ] {
        let mut game = ready_game();
        game.battlefield.clear();
        nissa_on_the_battlefield(&mut game, 85_100);
        let land = land_ready(&mut game, definition);

        tap_for(&mut game, land, ManaColor::Green, PlayerId::One);

        assert_eq!(
            game.players[0].mana_pool.green, 2,
            "{definition:?} is a Forest and doubles like one",
        );
    }
}
