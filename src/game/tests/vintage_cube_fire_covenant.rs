//! Fire Covenant: three mana and as much life as you can stand, split among
//! as many creatures as it takes.

use super::*;

/// The Covenant in hand with its three mana up, and `theirs` creatures on
/// the other side.
fn staged(theirs: &[CardDefinitionId]) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let creatures = theirs
        .iter()
        .map(|definition| {
            game.put_onto_battlefield(PlayerId::Two, *definition)
                .expect("cataloged")
        })
        .collect::<Vec<_>>();
    drain_pending(&mut game);
    let covenant = game
        .build_zone(PlayerId::One, &[cards::FIRE_COVENANT])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let covenant_id = covenant.id;
    game.players[0].hand.push(covenant);
    game.players[0].life = 20;
    game.players[1].life = 20;
    game.turns_started = [2, 1];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    (game, covenant_id, creatures)
}

fn casts(game: &Game, covenant: GameObjectId) -> Vec<CastChoices> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell { card, choices, .. } if card == covenant => Some(choices),
            _ => None,
        })
        .collect()
}

fn settle(game: &mut Game) {
    for _ in 0..16 {
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

fn damage_on(game: &Game, permanent: GameObjectId) -> Option<u16> {
    game.battlefield
        .iter()
        .find(|candidate| candidate.card.id == permanent)
        .map(|candidate| candidate.damage)
}

/// X is the life, not the mana: three mana offers every split its caster can
/// pay for.
#[test]
fn x_is_however_much_life_you_will_pay() {
    let (mut game, covenant, _) = staged(&[cards::GRIZZLY_BEARS]);

    let mut offered = casts(&game, covenant)
        .into_iter()
        .map(|choices| choices.x())
        .collect::<Vec<_>>();
    offered.sort_unstable();
    offered.dedup();
    // One is the floor rather than zero: every share of a divided total has
    // to be at least one damage, so an X of nothing has nothing to divide
    // and no way to fill the slot.
    assert_eq!(
        offered.first().copied(),
        Some(1),
        "one life is the smallest cast that does anything",
    );
    assert!(
        offered.contains(&20),
        "and every point of life you have is on offer: {offered:?}",
    );

    game.players[0].life = 2;
    let mut poorer = casts(&game, covenant)
        .into_iter()
        .map(|choices| choices.x())
        .collect::<Vec<_>>();
    poorer.sort_unstable();
    poorer.dedup();
    assert_eq!(poorer, vec![1, 2], "two life buys two damage and no more");
}

/// The damage is divided as its caster chooses, and the life is spent for
/// all of it.
#[test]
fn it_splits_its_damage_among_the_creatures_it_names() {
    let (mut game, covenant, creatures) = staged(&[cards::SERRA_ANGEL, cards::GRIZZLY_BEARS]);
    let (angel, bears) = (creatures[0], creatures[1]);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            let Action::CastSpell { card, choices, .. } = action else {
                return false;
            };
            *card == covenant
                && choices.x() == 3
                && choices.targets().iter().any(|selection| {
                    selection.amount_for(Target::Permanent(angel)) == Some(1)
                        && selection.amount_for(Target::Permanent(bears)) == Some(2)
                })
        })
        .expect("three life split one and two");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game);

    assert_eq!(game.players[0].life, 17, "three life, paid on casting");
    assert_eq!(damage_on(&game, angel), Some(1), "a 4/4 survives one");
    assert!(damage_on(&game, bears).is_none(), "and two kills the 2/2");
}

/// Creatures only: it cannot be aimed at a player, however much life is
/// paid for it.
#[test]
fn it_cannot_be_aimed_at_a_player() {
    let (game, covenant, _) = staged(&[cards::GRIZZLY_BEARS]);

    assert!(
        casts(&game, covenant).iter().all(|choices| {
            choices
                .targets()
                .iter()
                .flat_map(TargetSelection::targets)
                .all(|target| !matches!(target, Target::Player(_)))
        }),
        "no cast names a player",
    );
}

/// It reaches your own creatures too, which is what makes it a board wipe
/// rather than removal.
#[test]
fn it_reaches_both_sides_of_the_table() {
    let (mut game, covenant, creatures) = staged(&[cards::GRIZZLY_BEARS]);
    let theirs = creatures[0];
    let mine = game
        .put_onto_battlefield(PlayerId::One, cards::SAVANNAH_LIONS)
        .expect("cataloged");
    drain_pending(&mut game);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            let Action::CastSpell { card, choices, .. } = action else {
                return false;
            };
            *card == covenant
                && choices.x() == 4
                && choices.targets().iter().any(|selection| {
                    selection.amount_for(Target::Permanent(mine)) == Some(2)
                        && selection.amount_for(Target::Permanent(theirs)) == Some(2)
                })
        })
        .expect("four life split evenly across the table");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game);

    assert!(damage_on(&game, mine).is_none(), "your own 2/1 died too");
    assert!(damage_on(&game, theirs).is_none());
    assert_eq!(game.players[0].life, 16);
}

/// The life is a cost: it is spent as the spell is cast, whatever becomes of
/// the spell afterwards.
#[test]
fn the_life_is_paid_on_casting() {
    let (mut game, covenant, creatures) = staged(&[cards::GRIZZLY_BEARS]);
    let bears = creatures[0];

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            let Action::CastSpell { card, choices, .. } = action else {
                return false;
            };
            *card == covenant
                && choices.x() == 5
                && choices
                    .targets()
                    .iter()
                    .any(|selection| selection.amount_for(Target::Permanent(bears)) == Some(5))
        })
        .expect("five life at one creature");
    game.apply(PlayerId::One, cast).expect("it is cast");

    assert_eq!(game.players[0].life, 15, "paid while it is on the stack");
    assert_eq!(game.stack.len(), 1);
}
