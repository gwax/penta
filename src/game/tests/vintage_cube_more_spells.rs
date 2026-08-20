//! More spells cataloged for the Vintage Cube pool: the ones cast for
//! something other than their mana cost, cast a second time from the
//! graveyard, or cast with more than one mode at once.

use super::*;

/// The free cast is gated on whose turn it is, and the printed one is not.
/// A green card in hand pays for it only while someone else is the active
/// player.
#[test]
fn force_of_vigor_is_free_only_on_someone_elses_turn() {
    let free_cast_offered = |active: PlayerId| {
        let mut game = ready_game();
        game.active_player = active;
        let force = card(57_000, cards::FORCE_OF_VIGOR, PlayerId::One);
        let force_id = force.id;
        game.players[PlayerId::One.index()].hand.push(force);
        game.players[PlayerId::One.index()].hand.push(card(
            57_001,
            cards::BIRDS_OF_PARADISE,
            PlayerId::One,
        ));
        game.battlefield
            .push(creature(57_002, cards::BLACK_LOTUS, PlayerId::Two));
        game.legal_actions(PlayerId::One).into_iter().any(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if card == force_id && choices.costs().alternative().is_some())
        })
    };

    assert!(
        !free_cast_offered(PlayerId::One),
        "on your own turn there is no free cast, whatever is in hand",
    );
    assert!(
        free_cast_offered(PlayerId::Two),
        "on someone else's turn a green card pays for it",
    );
}

/// "Up to two" means the spell can take one, and "artifacts and/or
/// enchantments" means it does not care which.
#[test]
fn force_of_vigor_destroys_both_kinds_at_once() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.active_player = PlayerId::Two;
    let lotus = creature(57_100, cards::BLACK_LOTUS, PlayerId::Two);
    let lotus_id = lotus.card.id;
    game.battlefield.push(lotus);
    let arena = creature(57_101, cards::PHYREXIAN_ARENA, PlayerId::Two);
    let arena_id = arena.card.id;
    game.battlefield.push(arena);
    // Not an artifact or an enchantment, so never a legal target.
    let bears = creature(57_102, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);

    let force = card(57_103, cards::FORCE_OF_VIGOR, PlayerId::One);
    let force_id = force.id;
    game.players[PlayerId::One.index()].hand.push(force);
    game.players[PlayerId::One.index()].hand.push(card(
        57_104,
        cards::BIRDS_OF_PARADISE,
        PlayerId::One,
    ));

    assert!(
        !game.legal_actions(PlayerId::One).iter().any(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
            if *card == force_id
                && choices.targets().iter().any(|selection| {
                    selection.targets().contains(&Target::Permanent(bears_id))
                }))
        }),
        "a creature is not an artifact or an enchantment",
    );

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
            if *card == force_id
                && choices.costs().alternative().is_some()
                && choices.targets().iter().any(|selection| {
                    selection.targets().contains(&Target::Permanent(lotus_id))
                        && selection.targets().contains(&Target::Permanent(arena_id))
                }))
        })
        .expect("both halves of the board can go at once");
    game.apply(PlayerId::One, action).expect("it is cast");
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != lotus_id && permanent.card.id != arena_id),
        "the artifact and the enchantment are both destroyed",
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == bears_id),
        "and the creature is untouched",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].exile.len(),
        1,
        "the green card it spent was exiled, not discarded",
    );
}

/// Two damage, and then the same two again from the graveyard -- after which
/// the card is exiled rather than left to be flashed back twice.
#[test]
fn firebolt_burns_from_hand_and_once_more_from_the_graveyard() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    let bolt = card(73_000, cards::FIREBOLT, PlayerId::One);
    let bolt_id = bolt.id;
    game.players[0].hand.push(bolt);
    game.players[0].mana_pool.red = 1;
    let start = game.players[PlayerId::Two.index()].life;

    game.apply(
        PlayerId::One,
        cast_action(bolt_id, vec![Target::Player(PlayerId::Two)], Vec::new(), 0),
    )
    .expect("it is cast from hand");
    drain_pending(&mut game);
    assert_eq!(game.players[PlayerId::Two.index()].life, start - 2);
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::FIREBOLT),
        "and it lands in the graveyard, where the flashback lives",
    );

    let from_graveyard = game.players[0]
        .graveyard
        .iter()
        .find(|card| card.definition == cards::FIREBOLT)
        .expect("still there")
        .id;
    game.players[0].mana_pool.red = 1;
    game.players[0].mana_pool.colorless = 4;
    let flashback = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
            if *card == from_graveyard
                && choices.targets().iter().any(|selection| {
                    selection.targets().contains(&Target::Player(PlayerId::Two))
                }))
        })
        .expect("flashback is offered from the graveyard");
    game.apply(PlayerId::One, flashback).expect("it is cast");
    drain_pending(&mut game);

    assert_eq!(game.players[PlayerId::Two.index()].life, start - 4);
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .all(|card| card.definition != cards::FIREBOLT),
        "flashback exiles it rather than returning it",
    );
    assert_eq!(game.players[0].exile.len(), 1);
}

/// The chain is the opponent's to continue. Unlike Chain of Vapor, passing it
/// on costs nothing -- so what stops it is a player choosing to stop it, or
/// running out of cards to lose.
#[test]
fn chain_of_smog_discards_two_and_offers_the_chain_back() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::Two.index()].hand.clear();
    for (instance, definition) in [
        (75_000, cards::LIGHTNING_BOLT),
        (75_001, cards::SERRA_ANGEL),
        (75_002, cards::FOREST),
    ] {
        game.players[PlayerId::Two.index()]
            .hand
            .push(card(instance, definition, PlayerId::Two));
    }

    let chain = card(75_100, cards::CHAIN_OF_SMOG, PlayerId::One);
    let chain_id = chain.id;
    game.players[PlayerId::One.index()].hand.push(chain);
    game.players[PlayerId::One.index()].mana_pool.black = 1;
    game.players[PlayerId::One.index()].mana_pool.colorless = 1;
    game.priority = PlayerId::One;
    game.apply(
        PlayerId::One,
        cast_action(chain_id, vec![Target::Player(PlayerId::Two)], Vec::new(), 0),
    )
    .expect("it can name a player");

    // The discard is theirs to choose, so answer it for them.
    for _ in 0..8 {
        let Some(decision) = game.observe(PlayerId::Two).decision else {
            let player = game.priority;
            if game.apply(player, Action::PassPriority).is_err() {
                break;
            }
            continue;
        };
        if decision.prompt.contains("copy") {
            // The chain offer: this is what the test came for.
            assert_eq!(
                game.players[PlayerId::Two.index()].hand.len(),
                1,
                "two cards went first",
            );
            assert_eq!(game.players[PlayerId::Two.index()].graveyard.len(), 2);
            return;
        }
        let chosen = decision
            .options
            .iter()
            .take(decision.minimum)
            .map(|option| option.id)
            .collect::<Vec<_>>();
        game.apply(
            PlayerId::Two,
            Action::ChooseDecision {
                decision: decision.id,
                options: chosen,
            },
        )
        .expect("the discard choice is legal");
    }
    panic!("the chain was never offered back to the player who was hit");
}

/// The free cast is gated twice: on a Swamp, and on having the life to
/// spend. Both are checked before the option is offered rather than at
/// resolution, so an unpayable alternative never appears as a legal action.
#[test]
fn snuff_out_is_free_only_with_a_swamp_and_the_life_to_pay() {
    let free_offered = |swamp: bool, life: i16| {
        let mut game = ready_game();
        game.battlefield.clear();
        if swamp {
            game.battlefield
                .push(creature(79_000, cards::SWAMP, PlayerId::One));
        }
        game.players[PlayerId::One.index()].life = life;
        let snuff = card(79_001, cards::SNUFF_OUT, PlayerId::One);
        let snuff_id = snuff.id;
        game.players[PlayerId::One.index()].hand.push(snuff);
        game.battlefield
            .push(creature(79_002, cards::GRIZZLY_BEARS, PlayerId::Two));
        game.legal_actions(PlayerId::One).into_iter().any(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if card == snuff_id && choices.costs().alternative().is_some())
        })
    };

    assert!(
        free_offered(true, 20),
        "a Swamp and twenty life pays for it"
    );
    assert!(!free_offered(false, 20), "no Swamp, no free cast");
    // CR 118.4: life may be paid when the total is at least the amount, so
    // exactly four is payable and takes its controller to zero.
    assert!(free_offered(true, 4), "four life can pay four");
    assert!(
        !free_offered(true, 3),
        "and three cannot, so the option is not offered at all",
    );
}

/// Casting it for free costs the four life and kills what it names -- and it
/// will not name a black creature.
#[test]
fn snuff_out_pays_four_life_and_destroys_a_nonblack_creature() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.battlefield
        .push(creature(79_100, cards::SWAMP, PlayerId::One));
    let bears = creature(79_101, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    let djinn = creature(79_102, cards::JUZAM_DJINN, PlayerId::Two);
    let djinn_id = djinn.card.id;
    game.battlefield.push(djinn);
    game.players[PlayerId::One.index()].life = 20;

    let snuff = card(79_103, cards::SNUFF_OUT, PlayerId::One);
    let snuff_id = snuff.id;
    game.players[PlayerId::One.index()].hand.push(snuff);

    assert!(
        !game.legal_actions(PlayerId::One).iter().any(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
            if *card == snuff_id
                && choices.targets().iter().any(|selection| {
                    selection.targets().contains(&Target::Permanent(djinn_id))
                }))
        }),
        "a black creature is not a legal target",
    );

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
            if *card == snuff_id
                && choices.costs().alternative().is_some()
                && choices.targets().iter().any(|selection| {
                    selection.targets().contains(&Target::Permanent(bears_id))
                }))
        })
        .expect("the free cast can name the green creature");
    game.apply(PlayerId::One, cast).expect("it is cast");
    assert_eq!(
        game.players[PlayerId::One.index()].life,
        16,
        "the life is paid as the spell is cast",
    );
    drain_pending(&mut game);
    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != bears_id),
        "and the creature is destroyed",
    );
}

/// "If you control a Wizard as you cast this spell, you may choose two
/// instead" is read where the spell is offered, so without a Wizard the
/// two-mode selections are not legal actions at all.
#[test]
fn flame_of_anor_offers_a_second_mode_only_beside_a_wizard() {
    for (wizard, expected) in [(false, 3), (true, 6)] {
        let mut game = ready_game();
        game.battlefield.clear();
        if wizard {
            game.battlefield
                .push(creature(76_000, cards::VIVI_ORNITIER, PlayerId::One));
        }
        let flame = card(76_001, cards::FLAME_OF_ANOR, PlayerId::One);
        let flame_id = flame.id;
        game.players[0].hand.push(flame);
        game.battlefield
            .push(creature(76_002, cards::GRIZZLY_BEARS, PlayerId::Two));
        game.battlefield
            .push(creature(76_003, cards::BLACK_LOTUS, PlayerId::Two));
        game.players[0].mana_pool.blue = 1;
        game.players[0].mana_pool.red = 1;
        game.players[0].mana_pool.colorless = 1;

        // One selection can be offered several times over, once per legal
        // target, so the modes are what this counts rather than the actions.
        let mut selections = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .filter_map(|action| match action {
                Action::CastSpell { card, choices, .. } if card == flame_id => {
                    Some(choices.modes().to_vec())
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        selections.sort_unstable();
        selections.dedup();

        assert_eq!(
            selections.iter().filter(|modes| modes.len() == 1).count(),
            3,
            "all three modes are always on offer",
        );
        assert_eq!(
            selections.len(),
            expected,
            "with{} a Wizard on the battlefield",
            if wizard { "" } else { "out" },
        );
    }
}

/// Both chosen modes resolve, each against its own target: the draw and the
/// damage are one spell, not two.
#[test]
fn flame_of_anor_resolves_both_chosen_modes() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.battlefield
        .push(creature(76_010, cards::VIVI_ORNITIER, PlayerId::One));
    let bears = creature(76_011, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    let flame = card(76_012, cards::FLAME_OF_ANOR, PlayerId::One);
    let flame_id = flame.id;
    game.players[0].hand.push(flame);
    game.players[0].mana_pool.blue = 1;
    game.players[0].mana_pool.red = 1;
    game.players[0].mana_pool.colorless = 1;
    let before = game.players[0].hand.len();

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } if *card == flame_id => {
                choices.modes() == [ModeId(0), ModeId(2)]
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(bears_id))
            }
            _ => false,
        })
        .expect("drawing and burning is a legal pair beside a Wizard");
    game.apply(PlayerId::One, action).expect("it is cast");
    drain_pending(&mut game);

    // One card left the hand and two were drawn.
    assert_eq!(game.players[0].hand.len(), before + 1);
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == bears_id),
        "five damage kills a 2/2",
    );
}

/// "Pay X life" is a cost, so the casts on offer stop at the life its caster
/// actually has. Paying none is always available.
#[test]
fn toxic_deluge_is_offered_for_as_much_life_as_you_have() {
    for life in [3_i16, 20] {
        let mut game = ready_game();
        game.battlefield.clear();
        game.players[0].life = life;
        let deluge = card(77_000, cards::TOXIC_DELUGE, PlayerId::One);
        let deluge_id = deluge.id;
        game.players[0].hand.push(deluge);
        game.players[0].mana_pool.black = 1;
        game.players[0].mana_pool.colorless = 2;

        let mut offered = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .filter_map(|action| match action {
                Action::CastSpell { card, choices, .. } if card == deluge_id => Some(choices.x()),
                _ => None,
            })
            .collect::<Vec<_>>();
        offered.sort_unstable();

        assert_eq!(
            offered,
            (0..=u16::try_from(life).unwrap()).collect::<Vec<_>>(),
            "with {life} life",
        );
    }
}

/// The life is paid as the spell is cast, and the same X is what every
/// creature shrinks by. A creature whose toughness reaches zero dies.
#[test]
fn toxic_deluge_pays_its_life_and_shrinks_every_creature() {
    let mut game = ready_game();
    game.battlefield.clear();
    let bears = creature(77_010, cards::GRIZZLY_BEARS, PlayerId::One);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    let angel = creature(77_011, cards::SERRA_ANGEL, PlayerId::Two);
    let angel_id = angel.card.id;
    game.battlefield.push(angel);
    let deluge = card(77_012, cards::TOXIC_DELUGE, PlayerId::One);
    let deluge_id = deluge.id;
    game.players[0].hand.push(deluge);
    game.players[0].mana_pool.black = 1;
    game.players[0].mana_pool.colorless = 2;
    let before = game.players[0].life;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if *card == deluge_id && choices.x() == 3)
        })
        .expect("three life is affordable");
    game.apply(PlayerId::One, action).expect("it is cast");
    drain_pending(&mut game);

    assert_eq!(game.players[0].life, before - 3, "the life is paid");
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == bears_id),
        "a 2/2 does not survive -3/-3",
    );
    let angel = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == angel_id)
        .expect("a 4/4 does");
    assert_eq!(game.toughness(angel), Some(1));
}
