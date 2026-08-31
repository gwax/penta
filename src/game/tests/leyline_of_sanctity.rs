//! Leyline of Sanctity: hexproof on a player rather than on a permanent.
//!
//! What it stops is a spell that names its controller. What it does not stop
//! is anything that names their permanents, and nothing at all of their own.

use super::*;

/// Player Two behind a Leyline, with Player One holding `spell` and the mana
/// for anything in this file.
fn staged(spell: CardDefinitionId) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game.put_onto_battlefield(PlayerId::Two, cards::LEYLINE_OF_SANCTITY)
        .expect("cataloged");
    drain_pending(&mut game);
    let card = card(96_500, spell, PlayerId::One);
    let card_id = card.id;
    game.players[PlayerId::One.index()].hand.push(card);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 3);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 3);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 3);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, card_id)
}

/// Every target a cast of `spell` is offering.
fn offered(game: &Game, spell: GameObjectId) -> Vec<Target> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell { card, choices, .. } if card == spell => {
                Some(choices.iter_targets().copied().collect::<Vec<_>>())
            }
            _ => None,
        })
        .flatten()
        .collect()
}

/// A Bolt may still be pointed at their creature; it may not be pointed at
/// them.
#[test]
fn it_stops_the_burn_that_names_the_player_and_not_the_burn_that_names_a_creature() {
    let (mut game, bolt) = staged(cards::LIGHTNING_BOLT);
    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);

    let targets = offered(&game, bolt);
    assert!(
        targets.contains(&Target::Permanent(theirs)),
        "their creature has no hexproof of its own: {targets:?}",
    );
    assert!(
        !targets.contains(&Target::Player(PlayerId::Two)),
        "and they are off the list: {targets:?}",
    );
    assert!(
        targets.contains(&Target::Player(PlayerId::One)),
        "the caster is still a legal target for their own Bolt",
    );
}

/// "Spells or abilities your opponents control": their own spells still
/// reach them, which is what makes the Leyline free to play under.
#[test]
fn its_controller_may_still_target_themselves() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::Two.index()].hand.clear();
    game.put_onto_battlefield(PlayerId::Two, cards::LEYLINE_OF_SANCTITY)
        .expect("cataloged");
    drain_pending(&mut game);
    let scour = card(96_600, cards::THOUGHT_SCOUR, PlayerId::Two);
    let scour_id = scour.id;
    game.players[PlayerId::Two.index()].hand.push(scour);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Blue, 1);
    game.priority = PlayerId::Two;

    let library = game.players[PlayerId::Two.index()].library.len();
    game.apply(
        PlayerId::Two,
        cast_action(scour_id, vec![Target::Player(PlayerId::Two)], Vec::new(), 0),
    )
    .expect("hexproof stops opponents, not its own controller");
    drain_pending(&mut game);

    assert_eq!(
        game.players[PlayerId::Two.index()].library.len(),
        library - 3,
        "two milled and one drawn, all of it their own doing",
    );
}

/// A spell that names no target reaches them regardless: hexproof is about
/// being targeted and nothing else.
#[test]
fn an_untargeted_spell_reaches_them_anyway() {
    let (mut game, twister) = staged(cards::TIMETWISTER);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    game.players[PlayerId::Two.index()].hand.clear();
    game.players[PlayerId::Two.index()].hand.push(card(
        96_700,
        cards::LIGHTNING_BOLT,
        PlayerId::Two,
    ));

    game.apply(
        PlayerId::One,
        cast_action(twister, Vec::new(), Vec::new(), 0),
    )
    .expect("it names nobody, so there is nobody it cannot name");
    drain_pending(&mut game);

    assert_eq!(
        game.players[PlayerId::Two.index()].hand.len(),
        7,
        "the Leyline has nothing to say about a spell that does not target",
    );
}
