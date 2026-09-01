//! Bonecrusher Giant, and the three things an Adventure card needs: a spell
//! that exiles itself for later, a permission to cast the other half from
//! there, and a turn in which damage cannot be prevented.

use super::*;

/// Resolves whatever is on the stack, answering nothing.
fn resolve(game: &mut Game) {
    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
}

/// The one cast of `card` using the named play option, if it is on offer.
fn cast_with(game: &Game, card: GameObjectId, option: PlayOptionId) -> Option<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell {
                card: id, choices, ..
            } => *id == card && choices.play_option() == option,
            _ => false,
        })
}

/// Both halves are castable from hand, and only from hand: the creature is
/// not yet anywhere else.
#[test]
fn bonecrusher_offers_both_halves_from_hand() {
    let mut game = ready_game();
    game.battlefield.clear();
    let giant = card(80_000, cards::BONECRUSHER_GIANT, PlayerId::One);
    let giant_id = giant.id;
    game.players[0].hand.push(giant);
    game.battlefield
        .push(creature(80_001, cards::GRIZZLY_BEARS, PlayerId::Two));
    game.players[0].mana_pool.red = 1;
    game.players[0].mana_pool.colorless = 2;

    assert!(
        cast_with(&game, giant_id, PlayOptionId::DEFAULT).is_some(),
        "the Giant is castable",
    );
    assert!(
        cast_with(&game, giant_id, PlayOptionId(1)).is_some(),
        "and so is Stomp",
    );
}

/// Stomp exiles itself on an adventure, and its owner may then cast the
/// creature from exile -- as the creature, never as Stomp again.
#[test]
fn stomp_goes_on_an_adventure_and_the_giant_comes_back_from_exile() {
    let mut game = ready_game();
    game.battlefield.clear();
    let giant = card(80_010, cards::BONECRUSHER_GIANT, PlayerId::One);
    let giant_id = giant.id;
    game.players[0].hand.push(giant);
    let bears = creature(80_011, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    game.players[0].mana_pool.red = 1;
    game.players[0].mana_pool.colorless = 1;

    let stomp = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == giant_id
                    && choices.play_option() == PlayOptionId(1)
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(bears_id))
            }
            _ => false,
        })
        .expect("Stomp can point at the Bears");
    game.apply(PlayerId::One, stomp).expect("it is cast");
    resolve(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == bears_id),
        "two damage kills a 2/2",
    );
    let exiled = game.players[0]
        .exile
        .iter()
        .find(|card| card.definition == cards::BONECRUSHER_GIANT)
        .expect("the card is exiled rather than put in the graveyard");
    let exiled_id = exiled.id;

    game.players[0].mana_pool.red = 1;
    game.players[0].mana_pool.colorless = 2;
    assert!(
        cast_with(&game, exiled_id, PlayOptionId(1)).is_none(),
        "the adventure cannot be taken twice",
    );
    let giant = cast_with(&game, exiled_id, PlayOptionId::DEFAULT)
        .expect("the creature may be cast from exile");
    game.apply(PlayerId::One, giant).expect("it is cast");
    resolve(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::BONECRUSHER_GIANT),
        "and it arrives as a creature",
    );
    assert!(game.players[0].exile.is_empty(), "leaving exile behind it");
}

/// Targeting the Giant costs two life whether or not the spell works.
#[test]
fn bonecrusher_burns_whoever_targets_it() {
    let mut game = ready_game();
    game.battlefield.clear();
    let giant = creature(80_020, cards::BONECRUSHER_GIANT, PlayerId::One);
    let giant_id = giant.card.id;
    game.battlefield.push(giant);
    let bolt = card(80_021, cards::LIGHTNING_BOLT, PlayerId::Two);
    let bolt_id = bolt.id;
    game.players[1].hand.push(bolt);
    game.players[1].mana_pool.red = 1;
    game.priority = PlayerId::Two;
    let before = game.players[1].life;

    let action = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == bolt_id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(giant_id))
            }
            _ => false,
        })
        .expect("the Bolt can point at the Giant");
    game.apply(PlayerId::Two, action).expect("it is cast");
    resolve(&mut game);

    assert_eq!(
        game.players[1].life,
        before - 2,
        "the trigger burns the Bolt's controller",
    );
}

/// "Damage can't be prevented this turn" turns off what protection prevents
/// (CR 702.16e), so Stomp reaches a creature with protection from red.
#[test]
fn stomp_reaches_through_protection() {
    let mut game = ready_game();
    game.battlefield.clear();
    let giant = card(80_030, cards::BONECRUSHER_GIANT, PlayerId::One);
    let giant_id = giant.id;
    game.players[0].hand.push(giant);
    game.players[0].mana_pool.red = 1;
    game.players[0].mana_pool.colorless = 1;
    game.players[1].life = 20;

    let stomp = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == giant_id
                    && choices.play_option() == PlayOptionId(1)
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Player(PlayerId::Two))
            }
            _ => false,
        })
        .expect("Stomp can point at a player");
    game.apply(PlayerId::One, stomp).expect("it is cast");
    resolve(&mut game);

    assert_eq!(game.players[1].life, 18);
    assert!(
        game.damage_cannot_be_prevented_this_turn,
        "and the rule is in force for the rest of the turn",
    );
}

/// "If an Adventure spell leaves the stack in any way other than resolving
/// -- most likely by being countered -- that card won't be exiled and the
/// spell's controller won't be able to cast it as a permanent later."
#[test]
fn a_countered_stomp_takes_the_giant_with_it() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].graveyard.clear();
    let giant = card(80_030, cards::BONECRUSHER_GIANT, PlayerId::One);
    let giant_id = giant.id;
    game.players[0].hand.push(giant);
    let bears = creature(80_031, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    let counterspell = card(80_032, cards::COUNTERSPELL, PlayerId::Two);
    let counterspell_id = counterspell.id;
    game.players[1].hand.push(counterspell);
    game.players[0].mana_pool.red = 1;
    game.players[0].mana_pool.colorless = 1;
    game.players[1].mana_pool.blue = 2;

    let stomp = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == giant_id
                    && choices.play_option() == PlayOptionId(1)
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(bears_id))
            }
            _ => false,
        })
        .expect("Stomp can point at the Bears");
    game.apply(PlayerId::One, stomp).expect("it is cast");
    let on_stack = game.stack.last().expect("Stomp is on the stack").id;
    game.apply(PlayerId::One, Action::PassPriority)
        .expect("they get a word in");
    game.apply(
        PlayerId::Two,
        cast_action(
            counterspell_id,
            vec![Target::Spell(on_stack)],
            Vec::new(),
            0,
        ),
    )
    .expect("two blue answers it");
    resolve(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == bears_id),
        "the Bears never took the two damage",
    );
    assert!(
        game.players[0].exile.is_empty(),
        "and nothing was exiled: only a resolving Adventure exiles itself",
    );
    let buried = game.players[0]
        .graveyard
        .iter()
        .find(|card| card.definition == cards::BONECRUSHER_GIANT)
        .expect("the card is in its owner's graveyard");
    assert!(
        cast_with(&game, buried.id, PlayOptionId::DEFAULT).is_none(),
        "so there is no Giant to cast later",
    );
}

/// "That spell's controller" is whoever cast it, yours included: a Giant
/// Growth of your own aimed at your own Giant burns you for two.
#[test]
fn your_own_spell_aimed_at_the_giant_burns_you() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    let giant = creature(80_100, cards::BONECRUSHER_GIANT, PlayerId::One);
    let giant_id = giant.card.id;
    game.battlefield.push(giant);
    let growth = card(80_101, cards::GIANT_GROWTH, PlayerId::One);
    let growth_id = growth.id;
    game.players[PlayerId::One.index()].hand.push(growth);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 1);
    game.priority = PlayerId::One;
    let life = game.players[PlayerId::One.index()].life;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == growth_id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(giant_id))
            }
            _ => false,
        })
        .expect("your own creature is a legal target for your own pump");
    game.apply(PlayerId::One, action).expect("it is cast");
    resolve(&mut game);

    assert_eq!(
        game.players[PlayerId::One.index()].life,
        life - 2,
        "the clause names the spell's controller, and that is you",
    );
    let pumped = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == giant_id)
        .expect("he is still there");
    assert_eq!(
        (game.power(pumped), game.toughness(pumped)),
        (Some(7), Some(6)),
        "and the Growth still resolved",
    );
}

/// "Bonecrusher Giant's ability resolves before the spell that caused it to
/// trigger. It resolves even if that spell is countered." The two damage is
/// dealt while their removal is still waiting, and countering it takes
/// nothing back.
#[test]
fn the_burn_lands_before_the_spell_and_outlives_a_counter() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::Two.index()].hand.clear();
    let giant = creature(80_200, cards::BONECRUSHER_GIANT, PlayerId::One);
    let giant_id = giant.card.id;
    game.battlefield.push(giant);
    let bolt = card(80_201, cards::LIGHTNING_BOLT, PlayerId::Two);
    let bolt_id = bolt.id;
    game.players[PlayerId::Two.index()].hand.push(bolt);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Red, 1);
    let counter = card(80_202, cards::COUNTERSPELL, PlayerId::One);
    let counter_id = counter.id;
    game.players[PlayerId::One.index()].hand.push(counter);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 2);
    game.priority = PlayerId::Two;
    let theirs = game.players[PlayerId::Two.index()].life;

    let cast = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == bolt_id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(giant_id))
            }
            _ => false,
        })
        .expect("the Bolt can point at the Giant");
    game.apply(PlayerId::Two, cast).expect("it is cast");
    let bolt_spell = game
        .stack
        .iter()
        .next()
        .expect("the Bolt is under its own trigger")
        .id;

    // The trigger sits above the Bolt, so it resolves first.
    for _ in 0..8 {
        if game.players[PlayerId::Two.index()].life < theirs {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    assert_eq!(
        game.players[PlayerId::Two.index()].life,
        theirs - 2,
        "the two damage lands while their Bolt is still waiting",
    );

    game.priority = PlayerId::One;
    let answer = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == counter_id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Spell(bolt_spell))
            }
            _ => false,
        })
        .expect("a Counterspell answers it");
    game.apply(PlayerId::One, answer).expect("it is cast");
    resolve(&mut game);
    game.check_state_based_actions();

    assert_eq!(
        game.players[PlayerId::Two.index()].life,
        theirs - 2,
        "countering the spell takes none of it back",
    );
    let survivor = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == giant_id)
        .expect("the Giant is untouched");
    assert_eq!(survivor.damage, 0, "and took no damage of his own");
}
