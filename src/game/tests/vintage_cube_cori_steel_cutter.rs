//! Cori-Steel Cutter: every second spell of the turn is a hasty Monk, and
//! the Equipment walks itself over to it.

use super::*;

/// The Cutter on the battlefield with two Bolts in hand and mana for them.
fn staged() -> (Game, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.put_onto_battlefield(PlayerId::One, cards::CORI_STEEL_CUTTER)
        .expect("cataloged");
    let mut bolts = Vec::new();
    for _ in 0..3 {
        let card = game
            .build_zone(PlayerId::One, &[cards::LIGHTNING_BOLT])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        bolts.push(card.id);
        game.players[0].hand.push(card);
    }
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turns_started = [4, 4];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.players[1].life = 20;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 6);
    (game, bolts)
}

/// Casts a Bolt at the other player, answering the attach question with
/// `attach`.
fn bolt(game: &mut Game, bolt: GameObjectId, attach: bool) {
    game.priority = PlayerId::One;
    game.apply(
        PlayerId::One,
        cast_action(bolt, vec![Target::Player(PlayerId::Two)], Vec::new(), 0),
    )
    .expect("the Bolt is castable");
    settle(game, attach);
}

fn settle(game: &mut Game, attach: bool) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let wanted = if attach { "Do it" } else { "Decline" };
            let options = decision
                .options
                .iter()
                .find(|option| option.label == wanted)
                .map_or_else(
                    || {
                        decision
                            .options
                            .iter()
                            .map(|option| option.id)
                            .take(decision.minimum.max(1))
                            .collect()
                    },
                    |option| vec![option.id],
                );
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

fn monks(game: &Game) -> Vec<&Permanent> {
    game.battlefield
        .iter()
        .filter(|permanent| permanent.card.definition == ObjectKind::Token)
        .collect()
}

fn cutter(game: &Game) -> &Permanent {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::CORI_STEEL_CUTTER)
        .expect("the Cutter is there")
}

/// The first spell does nothing; the second makes the Monk.
#[test]
fn the_second_spell_makes_a_monk() {
    let (mut game, bolts) = staged();

    bolt(&mut game, bolts[0], true);
    assert!(monks(&game).is_empty(), "one spell is not two");

    bolt(&mut game, bolts[1], true);

    let monks = monks(&game);
    assert_eq!(monks.len(), 1, "the second spell made one");
    assert_eq!(game.power(monks[0]), Some(2), "1/1 plus the Equipment");
    assert!(
        game.permanent_has_executable_keyword(monks[0], KeywordAbility::Haste),
        "and it can attack the turn it arrives",
    );
}

/// Exactly the second: a third spell in the same turn makes nothing.
#[test]
fn the_third_spell_makes_nothing() {
    let (mut game, bolts) = staged();
    bolt(&mut game, bolts[0], true);
    bolt(&mut game, bolts[1], true);

    bolt(&mut game, bolts[2], true);

    assert_eq!(monks(&game).len(), 1, "only the second spell counts");
}

/// The attach is a "may": declining leaves the Equipment where it was.
#[test]
fn the_attach_may_be_declined() {
    let (mut game, bolts) = staged();
    bolt(&mut game, bolts[0], false);
    bolt(&mut game, bolts[1], false);

    let monks = monks(&game);
    assert_eq!(monks.len(), 1, "the Monk still arrives");
    assert_eq!(game.power(monks[0]), Some(1), "an unequipped 1/1");
    assert_eq!(cutter(&game).attached_to, None, "and nothing was equipped");
}

/// The Monk has prowess, which is what makes the third spell worth casting
/// after all.
#[test]
fn the_monk_has_prowess() {
    let (mut game, bolts) = staged();
    bolt(&mut game, bolts[0], true);
    bolt(&mut game, bolts[1], true);
    let monk = monks(&game)[0].card.id;

    bolt(&mut game, bolts[2], true);

    let monk = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == monk)
        .expect("it is still there");
    assert_eq!(
        game.power(monk),
        Some(3),
        "1/1, the Equipment, and prowess for the third Bolt",
    );
}
