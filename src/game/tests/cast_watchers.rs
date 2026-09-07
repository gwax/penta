//! Permanents that watch the stack. Each of these reads a property of the
//! spell that was just cast -- its colour, its type, its mana value, its
//! controller -- and a predicate that read the watching permanent instead,
//! or that failed open, would look like a card that always fires or never
//! does. So each is driven with a spell that should trigger it and one that
//! should not.

use super::*;

fn ready() -> Game {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[1].hand.clear();
    game.turns_started[PlayerId::One.index()] = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game
}

fn resolve(game: &mut Game) {
    for _ in 0..12 {
        drain_pending(game);
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let holder = game.priority;
        if game.apply(holder, Action::PassPriority).is_err() {
            break;
        }
    }
}

/// `caster` casts `definition` with every colour of mana available.
fn cast(game: &mut Game, caster: PlayerId, id: u32, definition: CardDefinitionId) {
    let held = card(id, definition, caster);
    let held_id = held.id;
    game.players[caster.index()].hand.push(held);
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        game.add_unrestricted_mana(caster, color, 4);
    }
    // Creature spells are sorcery-speed, so whoever is casting takes the turn.
    game.active_player = caster;
    game.step = Step::PrecombatMain;
    game.priority = caster;
    let chosen = game
        .legal_actions(caster)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == held_id))
        .expect("the spell is castable");
    game.apply(caster, chosen).expect("it is cast");
    resolve(game);
}

fn life(game: &Game) -> (i16, i16) {
    (game.players[0].life, game.players[1].life)
}

/// Spellshock bills whoever cast, not whoever owns the enchantment, so it is
/// driven from both seats.
#[test]
fn spellshock_bills_the_caster_from_either_seat() {
    let mut mine = ready();
    mine.battlefield
        .push(creature(82_000, cards::SPELLSHOCK, PlayerId::One));
    cast(&mut mine, PlayerId::One, 82_001, cards::GRIZZLY_BEARS);
    assert_eq!(
        life(&mine),
        (18, 20),
        "casting into my own Spellshock costs me the two"
    );

    let mut theirs = ready();
    theirs
        .battlefield
        .push(creature(82_010, cards::SPELLSHOCK, PlayerId::One));
    cast(&mut theirs, PlayerId::Two, 82_011, cards::GRIZZLY_BEARS);
    assert_eq!(
        life(&theirs),
        (20, 18),
        "and it bills the opponent just as readily"
    );
}

/// Havoc reads the spell's colour. A red creature spell from the same seat
/// is the control: same caster, same card type, wrong colour.
#[test]
fn havoc_reads_the_colour_of_the_spell_not_the_caster() {
    let mut white = ready();
    white
        .battlefield
        .push(creature(82_100, cards::HAVOC, PlayerId::One));
    cast(&mut white, PlayerId::Two, 82_101, cards::SAVANNAH_LIONS);
    assert_eq!(
        life(&white),
        (20, 18),
        "a white spell from the opponent costs them two"
    );

    let mut red = ready();
    red.battlefield
        .push(creature(82_110, cards::HAVOC, PlayerId::One));
    cast(&mut red, PlayerId::Two, 82_111, cards::RAGING_GOBLIN);
    assert_eq!(
        life(&red),
        (20, 20),
        "and a red one from the same seat costs them nothing"
    );
}

/// The Pillar reads mana value, so the two spells differ in nothing else.
#[test]
fn the_pyrostatic_pillar_only_catches_the_cheap_spell() {
    let mut cheap = ready();
    cheap
        .battlefield
        .push(creature(82_200, cards::PYROSTATIC_PILLAR, PlayerId::One));
    cast(&mut cheap, PlayerId::Two, 82_201, cards::RAGING_GOBLIN);
    assert_eq!(life(&cheap), (20, 18), "a one-drop is three or less");

    let mut dear = ready();
    dear.battlefield
        .push(creature(82_210, cards::PYROSTATIC_PILLAR, PlayerId::One));
    cast(&mut dear, PlayerId::Two, 82_211, cards::SERRA_ANGEL);
    assert_eq!(life(&dear), (20, 20), "and a five-drop walks past it");
}

/// The Presence reads both halves of "you cast an enchantment spell", so the
/// controls are an enchantment from the wrong seat and a creature from the
/// right one.
#[test]
fn enchantresss_presence_needs_your_enchantment_and_nothing_else() {
    let hand_after = |caster: PlayerId, spell: CardDefinitionId| {
        let mut game = ready();
        game.battlefield.push(creature(
            82_300,
            cards::ENCHANTRESS_S_PRESENCE,
            PlayerId::One,
        ));
        cast(&mut game, caster, 82_301, spell);
        game.players[0].hand.len()
    };

    assert_eq!(
        hand_after(PlayerId::One, cards::OPPRESSION),
        1,
        "my own enchantment draws me a card"
    );
    assert_eq!(
        hand_after(PlayerId::Two, cards::OPPRESSION),
        0,
        "theirs draws me nothing"
    );
    assert_eq!(
        hand_after(PlayerId::One, cards::GRIZZLY_BEARS),
        0,
        "and neither does my creature"
    );
}
