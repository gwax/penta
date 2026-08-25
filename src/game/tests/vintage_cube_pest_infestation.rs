//! Pest Infestation: two mana an artifact, and two Pests for each one.

use super::*;

/// The Infestation in hand with `mana` green available, and `theirs` on the
/// battlefield under Player Two.
fn staged(mana: u16, theirs: &[CardDefinitionId]) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let mut ids = Vec::new();
    for definition in theirs {
        ids.push(
            game.put_onto_battlefield(PlayerId::Two, *definition)
                .expect("cataloged"),
        );
    }
    let spell = game
        .build_zone(PlayerId::One, &[cards::PEST_INFESTATION])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let spell_id = spell.id;
    game.players[0].hand.push(spell);
    drain_pending(&mut game);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.players[0].life = 20;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, mana);
    (game, spell_id, ids)
}

fn settle(game: &mut Game) {
    for _ in 0..24 {
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

fn pests(game: &Game) -> usize {
    game.battlefield
        .iter()
        .filter(|permanent| is_token_with(permanent, tokens::pest()))
        .count()
}

/// Every X the spell can be cast for, given the mana on the table.
fn castable_x(game: &Game, spell: GameObjectId) -> Vec<u16> {
    let mut values = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell { card, choices, .. } if card == spell => Some(choices.x()),
            _ => None,
        })
        .collect::<Vec<_>>();
    values.sort_unstable();
    values.dedup();
    values
}

/// Casts it for `x`, aiming at `victims`. The action is taken from what the
/// engine offers rather than built here: which targets a given X may name is
/// the enumerator's business.
fn cast(game: &mut Game, spell: GameObjectId, x: u16, victims: &[GameObjectId]) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                let named = choices
                    .iter_targets()
                    .filter_map(|target| match target {
                        Target::Permanent(id) => Some(*id),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                *card == spell
                    && choices.x() == x
                    && named.len() == victims.len()
                    && victims.iter().all(|id| named.contains(id))
            }
            _ => false,
        })
        .expect("that cast is offered");
    game.apply(PlayerId::One, action).expect("it is castable");
    settle(game);
}

/// X is paid twice, so five mana buys an X of two.
#[test]
fn the_doubled_x_halves_what_you_get() {
    let (game, spell, _theirs) = staged(5, &[]);

    assert_eq!(
        castable_x(&game, spell),
        vec![0, 1, 2],
        "one green plus two per X",
    );
}

/// One artifact destroyed and two Pests for it.
#[test]
fn it_destroys_and_repopulates() {
    let (mut game, spell, theirs) = staged(3, &[cards::SOL_RING]);

    cast(&mut game, spell, 1, &theirs);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::SOL_RING),
        "the Ring is gone",
    );
    assert_eq!(pests(&game), 2, "twice X of one");
}

/// "Up to X": an X of two may name one artifact, and the Pests still come in
/// pairs however few were named.
#[test]
fn the_pests_do_not_depend_on_the_targets() {
    let (mut game, spell, theirs) = staged(5, &[cards::SOL_RING]);

    cast(&mut game, spell, 2, &theirs);

    assert_eq!(pests(&game), 4, "twice X, not twice the targets");
}

/// An X of zero is a legal cast that destroys nothing and makes nothing,
/// which is what "up to" and "twice X" both come to at zero.
#[test]
fn nothing_for_nothing() {
    let (mut game, spell, _theirs) = staged(1, &[]);

    cast(&mut game, spell, 0, &[]);

    assert_eq!(pests(&game), 0);
}

/// Enchantments as well as artifacts, and both in one cast.
#[test]
fn it_answers_either_kind() {
    let (mut game, spell, theirs) = staged(5, &[cards::SOL_RING, cards::MOAT]);

    cast(&mut game, spell, 2, &theirs);

    assert!(
        game.battlefield.iter().all(|permanent| !matches!(
            permanent.card.definition,
            d if d == cards::SOL_RING || d == cards::MOAT
        )),
        "the artifact and the enchantment both died",
    );
    assert_eq!(pests(&game), 4);
}

/// The Pests are the ones every Witherbloom card makes: a life on the way
/// out.
#[test]
fn a_pest_pays_a_life_when_it_dies() {
    let (mut game, spell, theirs) = staged(3, &[cards::SOL_RING]);
    cast(&mut game, spell, 1, &theirs);
    let pest = game
        .battlefield
        .iter()
        .find(|permanent| is_token_with(permanent, tokens::pest()))
        .expect("there are Pests")
        .card
        .id;
    let life = game.players[0].life;

    game.move_permanents_to_graveyard(&[pest]);
    settle(&mut game);

    assert_eq!(game.players[0].life, life + 1);
}
