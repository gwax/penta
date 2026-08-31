//! Thalia, Guardian of Thraben: a 2/1 first striker who charges everybody a
//! mana for everything that is not a creature.

use super::*;

/// Thalia on the battlefield under `controller`, with a Bolt and a Bears in
/// Player One's hand.
fn staged(controller: PlayerId) -> (Game, GameObjectId, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let thalia = creature(131_000, cards::THALIA_GUARDIAN_OF_THRABEN, controller);
    let thalia_id = thalia.card.id;
    game.battlefield.push(thalia);
    let bolt = card(131_100, cards::LIGHTNING_BOLT, PlayerId::One);
    let bolt_id = bolt.id;
    game.players[0].hand.push(bolt);
    let bears = card(131_101, cards::GRIZZLY_BEARS, PlayerId::One);
    let bears_id = bears.id;
    game.players[0].hand.push(bears);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, thalia_id, bolt_id, bears_id)
}

fn castable(game: &Game, spell: GameObjectId) -> bool {
    game.legal_actions(PlayerId::One)
        .iter()
        .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == spell))
}

/// One more mana for a noncreature spell, and nothing for a creature.
#[test]
fn she_taxes_the_spells_that_are_not_creatures() {
    let (mut game, _thalia, bolt, bears) = staged(PlayerId::One);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    assert!(!castable(&game, bolt), "one red is no longer enough");

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    assert!(castable(&game, bolt), "and one more mana pays her");

    game.players[0].mana_pool = ManaPool::default();
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    assert!(
        castable(&game, bears),
        "a creature spell pays her nothing at all",
    );
}

/// "Thalia's ability affects each spell that's not a creature spell,
/// including your own." Hers is not a tax on the other player.
#[test]
fn her_own_side_pays_the_same_tax() {
    for controller in [PlayerId::One, PlayerId::Two] {
        let (mut game, _thalia, bolt, _bears) = staged(controller);
        game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
        assert!(
            !castable(&game, bolt),
            "one red is short whoever controls her",
        );
        game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
        assert!(castable(&game, bolt), "and two pays for it");
    }
}

/// "The mana value of the spell remains unchanged, no matter what the total
/// cost to cast it was": a Bolt taxed to two mana is still a one-drop.
#[test]
fn the_tax_does_not_change_what_the_spell_is_worth() {
    let (mut game, _thalia, bolt, _bears) = staged(PlayerId::One);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    let blast = card(131_200, cards::SPELL_BLAST, PlayerId::Two);
    let blast_id = blast.id;
    game.players[1].hand.push(blast);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Blue, 1);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Colorless, 4);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == bolt))
        .expect("two mana casts the taxed Bolt");
    game.apply(PlayerId::One, cast).expect("it is cast");
    let on_stack = game.stack.last().expect("it is on the stack").id;
    game.priority = PlayerId::Two;

    let answers = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell { card, choices, .. }
                if card == blast_id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Spell(on_stack)) =>
            {
                Some(choices.x())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(answers, vec![1], "the printed cost is what it is worth");
}

/// First strike is printed on her, which is what makes a 2/1 awkward to
/// block.
#[test]
fn she_strikes_first() {
    let (game, thalia, _bolt, _bears) = staged(PlayerId::One);

    let permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == thalia)
        .expect("she is on the battlefield");
    assert!(game.permanent_has_executable_keyword(permanent, KeywordAbility::FirstStrike));
    assert_eq!(game.power(permanent), Some(2));
    assert_eq!(game.toughness(permanent), Some(1));
}

/// First strike doing what it is for: blocking a 2/1, she kills it before it
/// can answer and walks away whole.
#[test]
fn her_first_strike_kills_before_it_is_answered() {
    let (mut game, thalia, _bolt, _bears) = staged(PlayerId::One);
    let attacker = creature(131_200, cards::SAVANNAH_LIONS, PlayerId::Two);
    let attacker_id = attacker.card.id;
    game.battlefield.push(attacker);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.active_player = PlayerId::Two;
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.declare_attacker(attacker_id, AttackDefender::Player(PlayerId::One));
    game.finish_declaring_attackers();
    drain_pending(&mut game);

    game.step = Step::DeclareBlockers;
    game.declare_blocker(thalia, attacker_id);
    // The step rather than the single wave: first strike is a damage step of
    // its own, and dealing all of it at once would have them trade.
    game.advance_step();
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == attacker_id),
        "two damage first is enough for a 2/1",
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == thalia),
        "and it never got to hit back",
    );
}

/// The tax is on spells: a land is played rather than cast, and an ability
/// is activated rather than cast, so neither one owes her anything.
#[test]
fn she_taxes_neither_lands_nor_abilities() {
    let (mut game, _thalia, _bolt, _bears) = staged(PlayerId::One);
    let land = card(131_300, cards::MOUNTAIN, PlayerId::One);
    let land_id = land.id;
    game.players[0].hand.push(land);
    let key = game
        .put_onto_battlefield(PlayerId::One, cards::MANIFOLD_KEY)
        .expect("cataloged");
    let lotus = game
        .put_onto_battlefield(PlayerId::One, cards::BLACK_LOTUS)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.players[0].lands_played_this_turn = 0;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    assert!(
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::PlayLand { card, .. } if *card == land_id)),
        "a land is played and never cast",
    );

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    assert!(
        game.legal_actions(PlayerId::One).iter().any(|action| {
            matches!(action, Action::ActivateAbility { source, targets, .. }
                if *source == key
                    && targets
                        .iter()
                        .any(|slot| slot.targets().contains(&Target::Permanent(lotus))))
        }),
        "and a one-mana ability still costs one mana",
    );
}
