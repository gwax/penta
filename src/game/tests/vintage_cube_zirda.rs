//! Zirda, the Dawnwaker: two mana off every activated ability, and a tap
//! that walks something past a blocker.

use super::*;

fn staged(extra: &[CardDefinitionId]) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    let mut ids = Vec::new();
    for (index, definition) in extra.iter().enumerate() {
        let permanent = creature(
            109_000 + u32::try_from(index).expect("few permanents"),
            *definition,
            PlayerId::One,
        );
        ids.push(permanent.card.id);
        game.battlefield.push(permanent);
    }
    let zirda = game
        .put_onto_battlefield(PlayerId::One, cards::ZIRDA_THE_DAWNWAKER)
        .expect("cataloged");
    drain_pending(&mut game);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, zirda, ids)
}

/// A 3/3 for three, and the discount applies to its own ability: {1} minus
/// {2} floors at one mana, so it still costs {1}.
#[test]
fn its_own_ability_still_costs_one() {
    let (mut game, zirda, _) = staged(&[]);
    let fox = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == zirda)
        .expect("it is there");
    assert_eq!((game.power(fox), game.toughness(fox)), (Some(3), Some(3)));

    assert!(
        game.legal_actions(PlayerId::One).iter().all(
            |action| !matches!(action, Action::ActivateAbility { source, .. } if *source == zirda)
        ),
        "with no mana at all it is not activatable",
    );
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    assert!(
        game.legal_actions(PlayerId::One).iter().any(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == zirda)
        ),
        "the floor is one mana, and one mana pays it",
    );
}

/// A four-mana ability elsewhere costs two with Zirda out.
#[test]
fn it_takes_two_off_another_permanents_ability() {
    let (mut game, _, ids) = staged(&[cards::ICY_MANIPULATOR]);
    let icy = ids[0];
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == icy)
    {
        permanent.entered_controller_turn = 0;
    }
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);

    assert!(
        game.legal_actions(PlayerId::One).iter().any(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == icy)
        ),
        "a one-mana ability after the discount is payable with one mana",
    );
}

/// The tap stops a creature blocking for the turn.
#[test]
fn tapping_it_stops_a_blocker() {
    let (mut game, zirda, _) = staged(&[]);
    let blocker = creature(109_500, cards::SERRA_ANGEL, PlayerId::Two);
    let blocker_id = blocker.card.id;
    game.battlefield.push(blocker);
    let attacker = creature(109_501, cards::GRIZZLY_BEARS, PlayerId::One);
    let attacker_id = attacker.card.id;
    game.battlefield.push(attacker);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);

    let stop = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } => {
                *source == zirda
                    && targets
                        .iter()
                        .flat_map(crate::casting::TargetSelection::targets)
                        .any(|chosen| *chosen == Target::Permanent(blocker_id))
            }
            _ => false,
        })
        .expect("the Angel is a legal target");
    game.apply(PlayerId::One, stop).expect("it activates");
    drain_pending(&mut game);

    game.step = Step::DeclareAttackers;
    game.declare_attacker(attacker_id, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    drain_pending(&mut game);

    assert!(
        !game.legal_actions(PlayerId::Two).iter().any(|action| {
            matches!(action, Action::DeclareBlocker { blocker, .. } if *blocker == blocker_id)
        }),
        "it cannot block this turn",
    );
}

/// Every way of cycling `card` that is on offer.
fn cyclings(game: &Game, card: GameObjectId) -> Vec<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == card),
        )
        .collect()
}

/// "Abilities you activate" is not "abilities of permanents you control":
/// cycling a card in hand is an ability you activate, and Zirda discounts
/// it down to the printed floor.
#[test]
fn it_discounts_an_ability_activated_from_hand() {
    let (mut game, _, _) = staged(&[]);
    let card = game
        .build_zone(PlayerId::One, &[cards::MISCALCULATION])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let card_id = card.id;
    game.players[0].hand.push(card);

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    assert_eq!(
        cyclings(&game, card_id).len(),
        1,
        "cycling {{2}} costs {{1}} with the Fox out",
    );
}

/// Without Zirda the same card wants its printed two.
#[test]
fn cycling_costs_its_printed_two_without_the_fox() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let card = game
        .build_zone(PlayerId::One, &[cards::MISCALCULATION])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let card_id = card.id;
    game.players[0].hand.push(card);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    assert!(
        cyclings(&game, card_id).is_empty(),
        "one mana does not pay a cycling cost of two",
    );

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    assert_eq!(cyclings(&game, card_id).len(), 1, "two mana does");
}

/// "Effects that reduce the generic mana cost of an activation cost can't
/// reduce that cost's coloured mana requirements ... {1}{R} would become
/// {R}." A Combat Medic's {1}{W} keeps its white and loses its generic, and
/// one white is all it takes.
#[test]
fn the_discount_eats_generic_and_leaves_the_colour() {
    let (mut game, _, ids) = staged(&[cards::COMBAT_MEDIC]);
    let medic = ids[0];
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == medic)
    {
        permanent.entered_controller_turn = 0;
    }

    let offered = |game: &Game| {
        game.legal_actions(PlayerId::One).iter().any(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == medic),
        )
    };
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    assert!(
        !offered(&game),
        "a colourless mana pays for none of what is left",
    );

    game.players[0].mana_pool = ManaPool::default();
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    assert!(
        offered(&game),
        "but the white pip is the whole of the cost now",
    );
}

/// "Activating Zirda's last ability after a creature has blocked won't
/// remove the blocking creature from combat." The blocker stays where it
/// is; what the ability stops is a block not yet made.
#[test]
fn it_does_not_undo_a_block_already_made() {
    let (mut game, zirda, ids) = staged(&[cards::SAVANNAH_LIONS]);
    let attacker = ids[0];
    let blocker = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }

    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.declare_attacker(attacker, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    game.step = Step::DeclareBlockers;
    game.blockers_declared = false;
    game.apply(PlayerId::Two, Action::DeclareBlocker { blocker, attacker })
        .expect("the Bears block the Lions");
    game.apply(PlayerId::Two, Action::FinishDeclaringBlockers)
        .expect("the declaration finishes");

    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    let stop = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } => {
                *source == zirda
                    && targets
                        .iter()
                        .any(|selection| selection.targets() == [Target::Permanent(blocker)])
            }
            _ => false,
        })
        .expect("the Bears are a legal thing to name");
    game.apply(PlayerId::One, stop).expect("it activates");
    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == blocker && permanent.blocking.contains(&attacker)),
        "the block was already made and stands",
    );
}
