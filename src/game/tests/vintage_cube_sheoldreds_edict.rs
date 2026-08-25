//! Sheoldred's Edict: three edicts on one card, and picking the right one is
//! the whole skill.

use super::*;

/// Player One holding the Edict with two mana up, and Player Two holding
/// whatever `theirs` says.
fn staged(theirs: &[CardDefinitionId]) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let permanents = theirs
        .iter()
        .map(|definition| {
            game.put_onto_battlefield(PlayerId::Two, *definition)
                .expect("cataloged")
        })
        .collect::<Vec<_>>();
    drain_pending(&mut game);
    let edict = game
        .build_zone(PlayerId::One, &[cards::SHEOLDRED_S_EDICT])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let edict_id = edict.id;
    game.players[0].hand.push(edict);
    game.turns_started = [2, 1];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    (game, edict_id, permanents)
}

/// Casts the Edict choosing `mode`, then answers whatever it asks by taking
/// the first thing offered.
fn cast_mode(game: &mut Game, edict: GameObjectId, mode: u8) {
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == edict && choices.modes() == [ModeId(mode)]
            }
            _ => false,
        })
        .unwrap_or_else(|| panic!("mode {mode} is offered"));
    game.apply(PlayerId::One, cast).expect("it is cast");
    for _ in 0..16 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .first()
                .map(|option| vec![option.id])
                .unwrap_or_default();
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the choice accepts what it offered");
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

fn on_battlefield(game: &Game, permanent: GameObjectId) -> bool {
    game.battlefield
        .iter()
        .any(|candidate| candidate.card.id == permanent)
}

fn tokens(game: &Game, player: PlayerId) -> usize {
    game.battlefield
        .iter()
        .filter(|permanent| {
            permanent.controller == player && permanent.card.definition == ObjectKind::Token
        })
        .count()
}

/// All three modes are on offer, and exactly one is chosen.
#[test]
fn it_offers_one_of_three_modes() {
    let (game, edict, _) = staged(&[cards::GRIZZLY_BEARS]);

    let mut modes = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell { card, choices, .. } if card == edict => {
                Some(choices.modes().to_vec())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    modes.sort_unstable();
    modes.dedup();
    assert_eq!(
        modes,
        vec![vec![ModeId(0)], vec![ModeId(1)], vec![ModeId(2)]],
        "one mode at a time and no combinations",
    );
}

/// The first mode takes a real creature, and their token is no answer to it.
#[test]
fn the_nontoken_mode_ignores_tokens() {
    let (mut game, edict, permanents) = staged(&[cards::GRIZZLY_BEARS]);
    let bears = permanents[0];
    game.create_token(
        PlayerId::Two,
        crate::card::TokenCharacteristics::creature(&["Bear"], &[ManaColor::Green], 2, 2),
    );
    drain_pending(&mut game);

    cast_mode(&mut game, edict, 0);

    assert!(!on_battlefield(&game, bears), "the real creature is gone");
    assert_eq!(tokens(&game, PlayerId::Two), 1, "and the token is not");
}

/// The second mode is the mirror: the token goes and the creature stays.
#[test]
fn the_token_mode_ignores_real_creatures() {
    let (mut game, edict, permanents) = staged(&[cards::GRIZZLY_BEARS]);
    let bears = permanents[0];
    game.create_token(
        PlayerId::Two,
        crate::card::TokenCharacteristics::creature(&["Bear"], &[ManaColor::Green], 2, 2),
    );
    drain_pending(&mut game);

    cast_mode(&mut game, edict, 1);

    assert_eq!(tokens(&game, PlayerId::Two), 0, "the token is gone");
    assert!(on_battlefield(&game, bears), "and the creature is not");
}

/// The third mode names planeswalkers, and creatures are safe from it.
#[test]
fn the_planeswalker_mode_takes_a_planeswalker() {
    let (mut game, edict, permanents) =
        staged(&[cards::JACE_THE_MIND_SCULPTOR, cards::GRIZZLY_BEARS]);
    let (jace, bears) = (permanents[0], permanents[1]);

    cast_mode(&mut game, edict, 2);

    assert!(!on_battlefield(&game, jace), "the planeswalker is gone");
    assert!(on_battlefield(&game, bears), "and the creature is safe");
}

/// It never touches your own board, whichever mode is chosen.
#[test]
fn it_only_asks_your_opponents() {
    let (mut game, edict, permanents) = staged(&[cards::GRIZZLY_BEARS]);
    let theirs = permanents[0];
    let mine = game
        .put_onto_battlefield(PlayerId::One, cards::SAVANNAH_LIONS)
        .expect("cataloged");
    drain_pending(&mut game);

    cast_mode(&mut game, edict, 0);

    assert!(!on_battlefield(&game, theirs), "theirs was sacrificed");
    assert!(on_battlefield(&game, mine), "and yours was never asked");
}

/// A player with nothing the mode names simply gives up nothing.
#[test]
fn an_empty_board_gives_up_nothing() {
    let (mut game, edict, _) = staged(&[cards::MOUNTAIN]);

    cast_mode(&mut game, edict, 0);

    assert_eq!(
        game.battlefield
            .iter()
            .filter(|permanent| permanent.controller == PlayerId::Two)
            .count(),
        1,
        "a land is not a creature",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::SHEOLDRED_S_EDICT),
        "and the Edict resolved anyway",
    );
}
