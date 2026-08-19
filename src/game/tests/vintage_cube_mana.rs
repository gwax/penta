//! Mana sources cataloged for the Vintage Cube pool.

use super::*;

/// The Halfling's coloured mana is Cavern of Souls narrowed to a different
/// question: not a creature type, but whether the spell is legendary.
#[test]
fn the_halflings_colored_mana_only_pays_for_legendary_spells() {
    for (spell, castable) in [(cards::TIFA_LOCKHART, true), (cards::GRIZZLY_BEARS, false)] {
        let mut game = ready_game();
        game.battlefield.clear();
        let halfling = creature(74_000, cards::DELIGHTED_HALFLING, PlayerId::One);
        let halfling_id = halfling.card.id;
        game.battlefield.push(halfling);
        let spell_card = card(74_001, spell, PlayerId::One);
        let spell_id = spell_card.id;
        game.players[0].hand.push(spell_card);

        game.apply(
            PlayerId::One,
            Action::ActivateManaAbility {
                source: halfling_id,
                ability: mana_ability_for(&game, halfling_id, ManaColor::Green),
                color: ManaColor::Green,
                counters_removed: None,
                cost_object: None,
            },
        )
        .expect("it taps for a colour");
        game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);

        assert_eq!(
            game.legal_actions(PlayerId::One).iter().any(
                |action| matches!(action, Action::CastSpell { card, .. } if *card == spell_id)
            ),
            castable,
            "{spell:?} should{} be castable on Halfling mana",
            if castable { "" } else { " not" },
        );
    }
}

/// And what it does pay for cannot be countered.
#[test]
fn a_legendary_spell_paid_with_halfling_mana_cannot_be_countered() {
    let mut game = ready_game();
    game.battlefield.clear();
    let halfling = creature(74_100, cards::DELIGHTED_HALFLING, PlayerId::One);
    let halfling_id = halfling.card.id;
    game.battlefield.push(halfling);
    let tifa = card(74_101, cards::TIFA_LOCKHART, PlayerId::One);
    let tifa_id = tifa.id;
    game.players[0].hand.push(tifa);
    let counterspell = card(74_102, cards::COUNTERSPELL, PlayerId::Two);
    let counterspell_id = counterspell.id;
    game.players[1].hand.push(counterspell);

    game.apply(
        PlayerId::One,
        Action::ActivateManaAbility {
            source: halfling_id,
            ability: mana_ability_for(&game, halfling_id, ManaColor::Green),
            color: ManaColor::Green,
            counters_removed: None,
            cost_object: None,
        },
    )
    .expect("it taps for green");
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Blue, 2);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == tifa_id))
        .expect("a legendary spell is castable on this mana");
    game.apply(PlayerId::One, cast).expect("it is cast");
    assert!(
        game.stack
            .last()
            .expect("Tifa is on the stack")
            .applied_effects
            .iter()
            .any(|effect| {
                effect.effect == AppliedEffectDef::Rule(AppliedRuleDef::CannotBeCountered)
                    && effect
                        .source
                        .is_some_and(|source| source.object == halfling_id)
            }),
        "the rider rides on the spell, not on the Halfling",
    );

    game.apply(PlayerId::One, Action::PassPriority).unwrap();
    let target = game.stack.last().expect("still there").id;
    game.apply(
        PlayerId::Two,
        cast_action(counterspell_id, vec![Target::Spell(target)], Vec::new(), 0),
    )
    .expect("the counterspell is castable");
    pass_priority_pair(&mut game);
    assert_eq!(
        game.stack.len(),
        1,
        "the counterspell resolved and left her on the stack",
    );
    pass_priority_pair(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::TIFA_LOCKHART),
        "and she resolves anyway",
    );
}

/// The colourless half carries neither the restriction nor the rider.
#[test]
fn the_halflings_colorless_mana_is_ordinary() {
    let mut game = ready_game();
    game.battlefield.clear();
    let halfling = creature(74_200, cards::DELIGHTED_HALFLING, PlayerId::One);
    let halfling_id = halfling.card.id;
    game.battlefield.push(halfling);
    let bears = card(74_201, cards::GRIZZLY_BEARS, PlayerId::One);
    let bears_id = bears.id;
    game.players[0].hand.push(bears);

    game.apply(
        PlayerId::One,
        Action::ActivateManaAbility {
            source: halfling_id,
            ability: mana_ability_for(&game, halfling_id, ManaColor::Colorless),
            color: ManaColor::Colorless,
            counters_removed: None,
            cost_object: None,
        },
    )
    .expect("it taps for colourless");
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 1);

    assert!(
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == bears_id)),
        "a nonlegendary creature can be cast on the colourless mana",
    );
}

/// The life is a cost, not a trigger. City of Brass pays when it becomes
/// tapped, by anyone; the Confluence pays only when its own ability is
/// activated, and cannot be activated at all with no life to spare.
#[test]
fn mana_confluence_charges_a_life_as_a_cost_of_its_own_ability() {
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        let mut game = ready_game();
        game.battlefield.clear();
        let land = game
            .put_onto_battlefield(PlayerId::One, cards::MANA_CONFLUENCE)
            .expect("cataloged");
        game.players[PlayerId::One.index()].life = 20;

        game.apply(
            PlayerId::One,
            Action::ActivateManaAbility {
                source: land,
                ability: mana_ability_for(&game, land, color),
                color,
                counters_removed: None,
                cost_object: None,
            },
        )
        .unwrap_or_else(|error| panic!("it makes {color:?}: {error}"));
        assert_eq!(
            game.players[PlayerId::One.index()].mana_pool.amount(color),
            1
        );
        assert_eq!(
            game.players[PlayerId::One.index()].life,
            19,
            "one life for one mana",
        );
    }
}

/// Tapped by something else, it costs nothing -- the difference from City of
/// Brass, which would pay either way.
#[test]
fn mana_confluence_costs_nothing_when_something_else_taps_it() {
    let mut game = ready_game();
    game.battlefield.clear();
    let land = game
        .put_onto_battlefield(PlayerId::One, cards::MANA_CONFLUENCE)
        .expect("cataloged");
    game.players[PlayerId::One.index()].life = 20;

    game.tap_permanent(land);
    drain_pending(&mut game);

    assert_eq!(game.players[PlayerId::One.index()].life, 20);
    assert_eq!(game.players[PlayerId::One.index()].mana_pool.total(), 0);
}
