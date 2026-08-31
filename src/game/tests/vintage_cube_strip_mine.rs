//! Strip Mine: a colourless land that trades itself for any land at all.
//!
//! What separates it from its cousins is the missing word: the target is
//! "target land", so an Island is as good as a Cradle, and there is no
//! condition anywhere on the card.

use super::*;

/// Player One with a Strip Mine out since last turn.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    let mine = game
        .put_onto_battlefield(PlayerId::One, cards::STRIP_MINE)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, mine)
}

/// Every land it is offering to destroy right now.
fn targets(game: &Game, mine: GameObjectId) -> Vec<GameObjectId> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } if source == mine => Some(
                targets
                    .iter()
                    .flat_map(crate::casting::TargetSelection::targets)
                    .filter_map(|target| match target {
                        Target::Permanent(id) => Some(*id),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .flatten()
        .collect()
}

fn blow_up(game: &mut Game, mine: GameObjectId, victim: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } => {
                *source == mine
                    && targets
                        .iter()
                        .flat_map(crate::casting::TargetSelection::targets)
                        .any(|target| *target == Target::Permanent(victim))
            }
            _ => false,
        })
        .expect("that land is on offer");
    game.apply(PlayerId::One, action).expect("it activates");
    pass_priority_pair(game);
    drain_pending(game);
    game.check_state_based_actions();
}

/// The half that is played in every game it is drawn.
#[test]
fn it_taps_for_one_colorless() {
    let (mut game, mine) = staged();

    game.apply(
        PlayerId::One,
        Action::ActivateManaAbility {
            source: mine,
            ability: mana_ability_for(&game, mine, ManaColor::Colorless),
            color: ManaColor::Colorless,
            counters_removed: None,
            cost_object: None,
            combination: None,
            triggered_mana: None,
        },
    )
    .expect("it taps for colourless");

    assert_eq!(game.players[PlayerId::One.index()].mana_pool.colorless, 1);
    assert_eq!(
        game.players[PlayerId::One.index()].mana_pool.total(),
        1,
        "one mana and only one",
    );
}

/// "Target land" and nothing further: basics included, either side, and the
/// Strip Mine itself, which is a land standing on the battlefield when the
/// target is chosen.
#[test]
fn it_names_any_land_at_all() {
    let (mut game, mine) = staged();
    let basic = game
        .put_onto_battlefield(PlayerId::Two, cards::ISLAND)
        .expect("cataloged");
    let nonbasic = game
        .put_onto_battlefield(PlayerId::Two, cards::BADLANDS)
        .expect("cataloged");
    let ours = game
        .put_onto_battlefield(PlayerId::One, cards::FOREST)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    let offered = targets(&game, mine);
    assert!(
        offered.contains(&basic),
        "the basic is what the Wasteland cannot have: {offered:?}",
    );
    assert!(offered.contains(&nonbasic));
    assert!(offered.contains(&ours), "your own land is a land too");
    assert!(offered.contains(&mine), "and so is the Strip Mine");

    blow_up(&mut game, mine, basic);
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == basic),
        "the Island is gone",
    );
    assert!(
        game.players[PlayerId::Two.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::ISLAND),
        "and it went to its owner's graveyard",
    );
}

/// Naming itself is legal and buys nothing: the cost sacrifices it before
/// the ability resolves, so the ability is left with no target it can
/// destroy.
#[test]
fn naming_itself_destroys_nothing_else() {
    let (mut game, mine) = staged();
    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::ISLAND)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    blow_up(&mut game, mine, mine);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == mine),
        "it was sacrificed to pay for the ability",
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == theirs),
        "and the land it did not name is untouched",
    );
}

/// Summoning sickness is a creature rule: a Strip Mine played this turn taps
/// for its ability the moment it arrives, which is what makes it a
/// land-destruction spell for no mana at all.
#[test]
fn a_strip_mine_played_this_turn_may_be_used_at_once() {
    let (mut game, mine) = staged();
    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::BADLANDS)
        .expect("cataloged");
    drain_pending(&mut game);
    let arrived = game.turns_started[PlayerId::One.index()];
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == mine)
        .expect("it is there")
        .entered_controller_turn = arrived;
    game.priority = PlayerId::One;

    blow_up(&mut game, mine, theirs);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == theirs),
        "it arrived this turn and blew up a land anyway",
    );
}

/// The land it destroys may be regenerated: the ability says destroy, not
/// exile, and the record leaves that shield in place.
#[test]
fn what_it_destroys_may_be_regenerated() {
    let (mut game, mine) = staged();
    let ours = game
        .put_onto_battlefield(PlayerId::One, cards::FOREST)
        .expect("cataloged");
    drain_pending(&mut game);
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == ours)
        .expect("it is there")
        .regeneration_shields = 1;
    game.priority = PlayerId::One;

    blow_up(&mut game, mine, ours);

    let survivor = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == ours)
        .expect("the shield answered a destroy");
    assert_eq!(
        survivor.regeneration_shields, 0,
        "and the shield was spent doing it",
    );
}
