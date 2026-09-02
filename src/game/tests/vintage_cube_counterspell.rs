//! Counterspell: two blue for the whole of "counter target spell".
//!
//! The card is the cube's measuring stick and turns up as a tool in dozens
//! of other files; that an activated ability is no spell it may name is
//! pinned in `premodern_cycling`. What is here is the card read on its own:
//! when it may be cast at all, whose spells it reaches, and where what it
//! counters ends up.

use super::*;

/// Player One holding a Counterspell with the mana for it, and `theirs` in
/// Player Two's hand.
fn staged(theirs: Option<CardDefinitionId>) -> (Game, GameObjectId, Option<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[1].hand.clear();
    game.players[1].graveyard.clear();
    let counter = game
        .build_zone(PlayerId::One, &[cards::COUNTERSPELL])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let counter_id = counter.id;
    game.players[0].hand.push(counter);
    let theirs = theirs.map(|definition| {
        let card = game
            .build_zone(PlayerId::Two, &[definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        let id = card.id;
        game.players[1].hand.push(card);
        id
    });
    game.turns_started = [5, 5];
    game.active_player = PlayerId::Two;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::Two;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 2);
    for color in ManaColor::COLORS {
        game.add_unrestricted_mana(PlayerId::Two, color, 5);
    }
    (game, counter_id, theirs)
}

fn casts_of(game: &Game, player: PlayerId, spell: GameObjectId) -> Vec<Action> {
    game.legal_actions(player)
        .into_iter()
        .filter(|action| matches!(action, Action::CastSpell { card, .. } if *card == spell))
        .collect()
}

/// "Target spell" with nothing on the stack is no target at all: the card
/// is uncastable until somebody gives it something to answer.
#[test]
fn an_empty_stack_leaves_it_uncastable() {
    let (mut game, counter, _) = staged(None);
    game.priority = PlayerId::One;

    assert!(
        game.stack.is_empty(),
        "the fixture puts nothing on the stack",
    );
    assert!(
        casts_of(&game, PlayerId::One, counter).is_empty(),
        "two blue buys nothing with nothing to name",
    );
}

/// What it counters goes to its owner's graveyard rather than the
/// battlefield, however permanent the spell was.
#[test]
fn a_countered_permanent_spell_goes_to_its_owners_graveyard() {
    let (mut game, counter, angel) = staged(Some(cards::SERRA_ANGEL));
    let angel = angel.expect("staged with one");
    let cast = casts_of(&game, PlayerId::Two, angel)
        .into_iter()
        .next()
        .expect("five mana casts the Angel");
    game.apply(PlayerId::Two, cast).expect("it is cast");
    game.apply(PlayerId::Two, Action::PassPriority)
        .expect("they pass");

    let answer = casts_of(&game, PlayerId::One, counter)
        .into_iter()
        .next()
        .expect("a spell on the stack is what it names");
    game.apply(PlayerId::One, answer).expect("it is cast");
    drain_pending(&mut game);

    assert!(game.battlefield.is_empty(), "the Angel never arrived");
    assert!(
        game.players[1]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::SERRA_ANGEL),
        "it is in the graveyard of the player who owned it",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::COUNTERSPELL),
        "and the Counterspell went to its own",
    );
}

/// "Target spell" does not say whose. Your own spell is as legal a target as
/// theirs, which is how a Counterspell answers something aimed at it.
#[test]
fn it_may_name_your_own_spell() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let bolt = game
        .build_zone(PlayerId::One, &[cards::LIGHTNING_BOLT])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let bolt_id = bolt.id;
    game.players[0].hand.push(bolt);
    let counter = game
        .build_zone(PlayerId::One, &[cards::COUNTERSPELL])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let counter_id = counter.id;
    game.players[0].hand.push(counter);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 2);

    let cast = casts_of(&game, PlayerId::One, bolt_id)
        .into_iter()
        .next()
        .expect("one red casts the Bolt");
    game.apply(PlayerId::One, cast).expect("it is cast");
    let on_stack = game.stack.last().expect("it is on the stack").id;

    let answer = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == counter_id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Spell(on_stack))
            }
            _ => false,
        })
        .expect("your own spell is a legal target");
    game.apply(PlayerId::One, answer).expect("it is cast");
    drain_pending(&mut game);

    assert_eq!(
        game.players[1].life, 20,
        "the Bolt was countered before it could deal its three",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::LIGHTNING_BOLT),
        "and went to the graveyard uncast for anything",
    );
}

/// "Target spell", and an ability on the stack is not one. A Manifold Key's
/// untap waiting to resolve is an object on the stack like any other and
/// the Counterspell still has nothing to name -- which is the whole reason
/// the format prints Stifle separately.
#[test]
fn an_ability_on_the_stack_is_no_target_for_it() {
    let (mut game, counter, _) = staged(None);
    let key = game
        .put_onto_battlefield(PlayerId::One, cards::MANIFOLD_KEY)
        .expect("cataloged");
    let ring = game
        .put_onto_battlefield(PlayerId::One, cards::SOL_RING)
        .expect("cataloged");
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        if permanent.card.id == ring {
            permanent.tapped = true;
        }
    }
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    let untap =
        game.legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| match action {
                Action::ActivateAbility {
                    source, targets, ..
                } => {
                    *source == key
                        && targets.iter().any(|selection| {
                            selection.targets().iter().any(
                                |target| matches!(target, Target::Permanent(id) if *id == ring),
                            )
                        })
                }
                _ => false,
            })
            .expect("one of the Ring's mana pays the Key");
    game.apply(PlayerId::One, untap).expect("it activates");

    assert_eq!(game.stack.len(), 1, "the ability is waiting on the stack");
    assert!(
        casts_of(&game, PlayerId::One, counter).is_empty(),
        "and the Counterspell has nothing it may name",
    );
}

/// A counter war, which is what two of these in a format do to each other:
/// the Counterspell answering a Counterspell resolves first, so the one
/// underneath never counters anything and the creature it was aimed at
/// resolves.
#[test]
fn a_counterspell_may_be_countered_by_another() {
    let (mut game, mine, theirs) = staged(Some(cards::GRIZZLY_BEARS));
    let bears = theirs.expect("they are holding it");
    let second = game
        .build_zone(PlayerId::Two, &[cards::COUNTERSPELL])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let second_id = second.id;
    game.players[1].hand.push(second);

    let cast = casts_of(&game, PlayerId::Two, bears)
        .into_iter()
        .next()
        .expect("they cast the bear");
    game.apply(PlayerId::Two, cast).expect("it is cast");

    game.priority = PlayerId::One;
    let answer = casts_of(&game, PlayerId::One, mine)
        .into_iter()
        .next()
        .expect("two blue answers it");
    game.apply(PlayerId::One, answer).expect("it is cast");
    let mine_on_stack = game.stack.last().expect("yours is on top").id;

    game.priority = PlayerId::Two;
    let counter_war = casts_of(&game, PlayerId::Two, second_id)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { choices, .. }
                if choices.iter_targets().any(|target| *target == Target::Spell(mine_on_stack)))
        })
        .expect("their Counterspell may name yours");
    game.apply(PlayerId::Two, counter_war).expect("it is cast");

    for _ in 0..16 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::GRIZZLY_BEARS),
        "the bear arrived: what would have countered it was countered first",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::COUNTERSPELL),
        "yours is in your graveyard, countered",
    );
    assert!(
        game.players[1]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::COUNTERSPELL),
        "and theirs in theirs, having resolved",
    );
}
