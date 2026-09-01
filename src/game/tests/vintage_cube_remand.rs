//! Remand: two mana that buys a turn and replaces itself. What it answers
//! comes back, so it is tempo rather than an answer.

use super::*;

/// Player Two casting a spell, Player One holding the Remand.
fn staged(theirs: CardDefinitionId) -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    game.players[1].hand.clear();
    game.players[1].graveyard.clear();
    for (index, definition) in [cards::MOUNTAIN, cards::FOREST].into_iter().enumerate() {
        let id = 283_000 + u32::try_from(index).expect("two cards");
        game.players[0]
            .library
            .push(card(id, definition, PlayerId::One));
    }
    let remand = game
        .build_zone(PlayerId::One, &[cards::REMAND])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let remand_id = remand.id;
    game.players[0].hand.push(remand);
    let spell = game
        .build_zone(PlayerId::Two, &[theirs])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let spell_id = spell.id;
    game.players[1].hand.push(spell);
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::Two;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::Two;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 2);
    for color in ManaColor::COLORS {
        game.add_unrestricted_mana(PlayerId::Two, color, 5);
    }
    (game, remand_id, spell_id)
}

fn settle(game: &mut Game) {
    for _ in 0..32 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .map(|option| option.id)
                .take(decision.minimum.max(1))
                .collect();
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the offered choice is legal");
            continue;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    game.check_state_based_actions();
}

/// Player Two casts their spell; Player One answers it with the Remand.
fn cast_and_answer(game: &mut Game, remand: GameObjectId, spell: GameObjectId) {
    let cast = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == spell))
        .expect("they have the mana");
    game.apply(PlayerId::Two, cast)
        .expect("their spell is cast");
    for _ in 0..4 {
        if game.priority == PlayerId::One {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    let answer = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == remand))
        .expect("two mana answers it");
    game.apply(PlayerId::One, answer).expect("it is cast");
    settle(game);
}

/// The countered spell goes back to its owner's hand, not their graveyard.
#[test]
fn the_countered_card_goes_back_to_hand() {
    let (mut game, remand, spell) = staged(cards::SERRA_ANGEL);

    cast_and_answer(&mut game, remand, spell);

    assert!(game.battlefield.is_empty(), "the creature never resolved");
    assert_eq!(
        game.players[1]
            .hand
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::SERRA_ANGEL],
        "and they have it back",
    );
    assert!(game.players[1].graveyard.is_empty());
}

/// And Remand replaces itself.
#[test]
fn it_draws_a_card() {
    let (mut game, remand, spell) = staged(cards::SERRA_ANGEL);

    cast_and_answer(&mut game, remand, spell);

    assert_eq!(game.players[0].hand.len(), 1, "one card drawn");
    assert_eq!(game.players[0].library.len(), 1);
}

/// The Remand itself is an ordinary spell and goes to its own graveyard.
#[test]
fn the_remand_goes_to_its_own_graveyard() {
    let (mut game, remand, spell) = staged(cards::SERRA_ANGEL);

    cast_and_answer(&mut game, remand, spell);

    assert_eq!(
        game.players[0]
            .graveyard
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::REMAND],
    );
}

/// It answers an instant the same way: what it names is any spell.
#[test]
fn it_answers_an_instant_too() {
    let (mut game, remand, spell) = staged(cards::LIGHTNING_BOLT);

    cast_and_answer(&mut game, remand, spell);

    assert_eq!(game.players[0].life, 20, "no damage was dealt");
    assert_eq!(
        game.players[1]
            .hand
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::LIGHTNING_BOLT],
    );
}

/// "Remand can target a spell that can't be countered. That spell won't be
/// countered or returned to its owner's hand, but you'll draw a card."
#[test]
fn it_still_draws_off_a_spell_it_cannot_counter() {
    let (mut game, remand, spell) = staged(cards::SUPREME_VERDICT);
    game.battlefield
        .push(creature(283_100, cards::GRIZZLY_BEARS, PlayerId::One));
    let library = game.players[0].library.len();

    cast_and_answer(&mut game, remand, spell);

    assert!(
        game.battlefield.is_empty(),
        "the Verdict resolved and swept the board",
    );
    assert!(
        game.players[1].hand.is_empty(),
        "it was not put back into their hand",
    );
    assert_eq!(
        game.players[1]
            .graveyard
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::SUPREME_VERDICT],
        "it went where a resolved sorcery goes",
    );
    assert_eq!(
        game.players[0].library.len(),
        library - 1,
        "and the Remand drew its card either way",
    );
}

/// "If you target a card that was cast with flashback with Remand, the card
/// will still be exiled." Flashback's own replacement is what moves it, and
/// it wins over "put it into its owner's hand instead".
#[test]
fn a_flashback_spell_is_exiled_rather_than_returned() {
    let (mut game, remand, _unused) = staged(cards::GRIZZLY_BEARS);
    game.players[1].hand.clear();
    let flashed = game
        .build_zone(PlayerId::Two, &[cards::FEELING_OF_DREAD])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let flashed_id = flashed.id;
    game.players[1].graveyard.push(flashed);

    cast_and_answer(&mut game, remand, flashed_id);

    assert!(
        game.players[1].hand.is_empty(),
        "the counter did not put it back in their hand",
    );
    assert!(
        game.players[1].graveyard.is_empty(),
        "and it did not stay in the graveyard either",
    );
    assert!(
        game.players[1]
            .exile
            .iter()
            .any(|card| card.definition == cards::FEELING_OF_DREAD),
        "flashback exiles what it was cast from wherever the spell ends up",
    );
}

/// The draw is a second clause of one spell, not a separate one: if the
/// spell Remand named is gone before Remand resolves, Remand is countered
/// for having no legal targets (CR 608.2b) and draws nothing. That is the
/// other side of drawing off a spell it merely fails to counter.
#[test]
fn a_fizzled_remand_draws_nothing() {
    let (mut game, remand, spell) = staged(cards::GRIZZLY_BEARS);
    let counterspell = card(283_100, cards::COUNTERSPELL, PlayerId::One);
    let counterspell_id = counterspell.id;
    game.players[0].hand.push(counterspell);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 2);

    let cast = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == spell))
        .expect("they have the mana");
    game.apply(PlayerId::Two, cast)
        .expect("their spell is cast");
    for _ in 0..4 {
        if game.priority == PlayerId::One {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }

    // Remand names the Bears, and then a Counterspell held behind it names
    // the same Bears and resolves first.
    let bears = game
        .stack
        .objects
        .iter()
        .find(|object| object.card.definition == cards::GRIZZLY_BEARS)
        .expect("their spell is on the stack")
        .id;
    for blue in [remand, counterspell_id] {
        let cast =
            game.legal_actions(PlayerId::One)
                .into_iter()
                .find(|action| match action {
                    Action::CastSpell { card, choices, .. } => {
                        *card == blue
                            && choices.targets().iter().any(|selection| {
                                selection.targets().contains(&Target::Spell(bears))
                            })
                    }
                    _ => false,
                })
                .expect("both name the Bears");
        game.apply(PlayerId::One, cast).expect("it is cast");
    }
    settle(&mut game);

    assert!(
        game.players[1]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS),
        "the Counterspell got there first, so the Bears went to the graveyard",
    );
    assert!(
        game.players[1].hand.is_empty(),
        "and never came back to hand",
    );
    assert_eq!(
        game.players[0].library.len(),
        2,
        "Remand found nothing to counter and so did nothing at all",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::REMAND),
        "it was countered by the game rules on the way",
    );
}

/// "Target spell" names any spell, your own included: a Remand aimed at your
/// own spell buys it back rather than losing it, which is what you do when
/// the alternative is losing it to their answer.
#[test]
fn it_may_answer_your_own_spell() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    game.players[0]
        .library
        .push(card(283_500, cards::MOUNTAIN, PlayerId::One));
    let angel = game
        .build_zone(PlayerId::One, &[cards::SERRA_ANGEL])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let angel_id = angel.id;
    game.players[0].hand.push(angel);
    let remand = game
        .build_zone(PlayerId::One, &[cards::REMAND])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let remand_id = remand.id;
    game.players[0].hand.push(remand);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 2);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 2);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 3);
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == angel_id))
        .expect("five mana casts the Angel");
    game.apply(PlayerId::One, cast).expect("it is cast");
    let on_stack = game.stack.last().expect("it is on the stack").id;

    let answer = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == remand_id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Spell(on_stack))
            }
            _ => false,
        })
        .expect("your own spell is a legal target");
    game.apply(PlayerId::One, answer).expect("it is cast");
    settle(&mut game);

    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::SERRA_ANGEL),
        "the Angel came back to your hand",
    );
    assert!(game.battlefield.is_empty(), "rather than resolving");
}

/// Resolves the top of the stack until a copy is sitting on it, answering
/// whatever is asked on the way, and hands priority to the other seat.
fn resolve_into_a_copy(game: &mut Game) -> GameObjectId {
    for _ in 0..12 {
        if game.stack.iter().any(|object| object.is_copy) {
            break;
        }
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options: vec![decision.options[0].id],
                },
            )
            .expect("the offered answer is legal");
            continue;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    let copy = game
        .stack
        .iter()
        .find(|object| object.is_copy)
        .expect("the Fork made a copy")
        .id;
    // The active player has priority after their own Fork resolves; the
    // Remand is answered from the other seat.
    for _ in 0..4 {
        if game.priority == PlayerId::One {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    copy
}

/// A copy on the stack is not a card, so there is no owner's hand to put it
/// into: countering it makes it cease to exist. The draw happens all the
/// same.
#[test]
fn a_copy_it_counters_ceases_to_exist() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[1].hand.clear();
    game.players[0].library.clear();
    game.players[0]
        .library
        .push(card(283_600, cards::MOUNTAIN, PlayerId::One));
    let bolt = card(283_601, cards::LIGHTNING_BOLT, PlayerId::Two);
    let bolt_id = bolt.id;
    game.players[1].hand.push(bolt);
    let fork = card(283_602, cards::FORK, PlayerId::Two);
    let fork_id = fork.id;
    game.players[1].hand.push(fork);
    let remand = game
        .build_zone(PlayerId::One, &[cards::REMAND])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let remand_id = remand.id;
    game.players[0].hand.push(remand);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::Two;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::Two;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 2);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Red, 4);

    game.apply(
        PlayerId::Two,
        cast_action(bolt_id, vec![Target::Player(PlayerId::One)], Vec::new(), 0),
    )
    .expect("the Bolt is cast");
    let original = game.stack.last().expect("it is on the stack").id;
    game.apply(
        PlayerId::Two,
        cast_action(fork_id, vec![Target::Spell(original)], Vec::new(), 0),
    )
    .expect("the Fork is cast at it");
    let copy = resolve_into_a_copy(&mut game);
    let hand = game.players[0].hand.len();
    let answer = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == remand_id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Spell(copy))
            }
            _ => false,
        })
        .expect("a copy on the stack is a spell it may name");
    game.apply(PlayerId::One, answer).expect("it is cast");
    for _ in 0..8 {
        if !game.stack.iter().any(|object| object.id == copy) {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }

    assert!(
        !game.stack.iter().any(|object| object.id == copy),
        "the copy was countered",
    );
    assert!(
        game.players[1].hand.is_empty(),
        "and there was no card to put into anybody's hand",
    );
    assert_eq!(
        game.players[0].hand.len(),
        hand,
        "the Remand left the hand and its draw replaced it",
    );
}
