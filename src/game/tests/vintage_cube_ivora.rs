//! Ivora, Insatiable Heir and the Blood token her shared trigger creates.

use super::*;

/// Ivora's two clauses feed each other: the Blood she makes is spent by
/// discarding, and the discard is what grows her.
#[test]
fn ivora_makes_blood_on_arrival_and_grows_on_any_discard() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    let ivora = game
        .put_onto_battlefield(PlayerId::One, cards::IVORA_INSATIABLE_HEIR)
        .expect("cataloged");
    drain_pending(&mut game);

    let blood = game
        .battlefield
        .iter()
        .find(|permanent| is_token_with(permanent, tokens::blood()))
        .expect("entering made a Blood token")
        .card
        .id;
    let size = |game: &Game| {
        let ivora = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == ivora)
            .expect("she is still there");
        (game.power(ivora), game.toughness(ivora))
    };
    assert_eq!(size(&game), (Some(1), Some(1)));

    // Spending the Blood discards a card, and that discard is a discard.
    game.players[PlayerId::One.index()]
        .hand
        .push(card(69_000, cards::FOREST, PlayerId::One));
    game.players[PlayerId::One.index()].mana_pool.colorless = 1;
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == blood))
        .expect("the Blood token can be spent");
    game.apply(PlayerId::One, action).expect("it activates");
    drain_pending(&mut game);

    assert_eq!(
        size(&game),
        (Some(2), Some(2)),
        "the discard paid as a cost still grows her",
    );
    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != blood),
        "and the token sacrificed itself to do it",
    );
}

/// Combat damage is the other way in, and it is the same printed ability --
/// which is why it has to be combat damage rather than any damage at all.
#[test]
fn ivora_makes_a_second_blood_only_when_she_connects_in_combat() {
    let mut game = ready_game();
    game.battlefield.clear();
    let ivora = creature(69_100, cards::IVORA_INSATIABLE_HEIR, PlayerId::One);
    let ivora_id = ivora.card.id;
    game.battlefield.push(ivora);

    let bloods = |game: &Game| {
        game.battlefield
            .iter()
            .filter(|permanent| is_token_with(permanent, tokens::blood()))
            .count()
    };

    game.damage_target_from(Some(ivora_id), Some(Target::Player(PlayerId::Two)), 1);
    drain_pending(&mut game);
    assert_eq!(
        bloods(&game),
        0,
        "damage that is not combat damage does nothing"
    );

    game.step = Step::DeclareAttackers;
    game.declare_attacker(ivora_id, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    drain_pending(&mut game);
    game.deal_combat_damage();
    drain_pending(&mut game);

    assert_eq!(bloods(&game), 1, "connecting in combat makes another Blood");
}

/// Ivora with `counters` on her, ready to attack.
fn attacking_ivora(counters: u16) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[1].hand.clear();
    let mut ivora = creature(69_200, cards::IVORA_INSATIABLE_HEIR, PlayerId::One);
    ivora.set_counters(CounterKind::PlusOnePlusOne, counters);
    let ivora_id = ivora.card.id;
    game.battlefield.push(ivora);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, ivora_id)
}

fn bloods(game: &Game) -> usize {
    game.battlefield
        .iter()
        .filter(|permanent| is_token_with(permanent, tokens::blood()))
        .count()
}

/// "Whenever *you* discard a card": their discard is not yours, so a Hymn
/// aimed across the table grows her by nothing.
#[test]
fn their_discard_does_not_grow_her() {
    let (mut game, ivora) = attacking_ivora(0);
    game.players[1]
        .hand
        .push(card(69_300, cards::FOREST, PlayerId::Two));
    game.players[1]
        .hand
        .push(card(69_301, cards::MOUNTAIN, PlayerId::Two));
    let hymn = card(69_302, cards::HYMN_TO_TOURACH, PlayerId::One);
    let hymn_id = hymn.id;
    game.players[0].hand.push(hymn);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 2);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == hymn_id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Player(PlayerId::Two))
            }
            _ => false,
        })
        .expect("two black casts it at them");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);

    assert!(game.players[1].hand.is_empty(), "they discarded both cards");
    let size = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == ivora)
        .expect("she is there");
    assert_eq!(
        (game.power(size), game.toughness(size)),
        (Some(1), Some(1)),
        "and she is the size she was: the clause reads your discards",
    );
}

/// "Whenever it deals combat damage to a player": trample is what carries
/// that past a blocker, and a blocker big enough to soak it all leaves
/// nothing to trigger on.
#[test]
fn only_the_damage_that_reaches_the_player_makes_blood() {
    for (blocker, expected) in [(cards::GRIZZLY_BEARS, 1), (cards::GIANT_SPIDER, 0)] {
        // A 4/4 Ivora: two through a 2/2, and none at all past a 2/4.
        let (mut game, ivora) = attacking_ivora(3);
        let wall = game
            .put_onto_battlefield(PlayerId::Two, blocker)
            .expect("cataloged");
        drain_pending(&mut game);
        for permanent in &mut game.battlefield {
            permanent.entered_controller_turn = 0;
        }
        game.step = Step::DeclareAttackers;
        game.attackers_declared = false;
        game.declare_attacker(ivora, AttackDefender::Player(PlayerId::Two));
        game.finish_declaring_attackers();
        drain_pending(&mut game);
        game.step = Step::DeclareBlockers;
        game.declare_blocker(wall, ivora);
        game.finish_declaring_blockers();
        drain_pending(&mut game);
        game.deal_combat_damage();
        drain_pending(&mut game);
        game.check_state_based_actions();

        assert_eq!(
            bloods(&game),
            expected,
            "{blocker:?} standing in front of a 4/4 trampler",
        );
    }
}
