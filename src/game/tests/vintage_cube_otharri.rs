//! Otharri, Suns' Glory: a hasty lifelinking flier that pays out more every
//! time it connects, and buys itself back with what it left behind.

use super::*;

/// Otharri attacking-ready on the battlefield, with `others` beside him.
fn staged(others: &[CardDefinitionId]) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    let otharri = game
        .put_onto_battlefield(PlayerId::One, cards::OTHARRI_SUNS_GLORY)
        .expect("cataloged");
    let mut ids = Vec::new();
    for definition in others {
        ids.push(
            game.put_onto_battlefield(PlayerId::One, *definition)
                .expect("cataloged"),
        );
    }
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [2, 1];
    drain_pending(&mut game);
    game.players[0].life = 20;
    game.players[1].life = 20;
    game.active_player = PlayerId::One;
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.priority = PlayerId::One;
    (game, otharri, ids)
}

/// Read through the observation rather than off the player: what a client
/// can see is what the counter is for.
fn experience(game: &Game) -> u16 {
    game.observe(PlayerId::One).counters[PlayerId::One.index()]
        .iter()
        .find(|counter| counter.name == "experience")
        .map_or(0, |counter| counter.count)
}

fn rebels(game: &Game) -> Vec<&Permanent> {
    game.battlefield
        .iter()
        .filter(|permanent| permanent.card.definition == ObjectKind::Token)
        .collect()
}

fn attack(game: &mut Game, otharri: GameObjectId) {
    game.apply(
        PlayerId::One,
        Action::DeclareAttacker {
            attacker: otharri,
            defender: AttackDefender::Player(PlayerId::Two),
        },
    )
    .expect("haste lets him attack");
    game.apply(PlayerId::One, Action::FinishDeclaringAttackers)
        .expect("the declaration finishes");
    drain_pending(game);
}

/// The first attack: a counter, then one Rebel for it, tapped and already
/// attacking.
#[test]
fn attacking_pays_an_experience_counter_and_a_rebel() {
    let (mut game, otharri, _) = staged(&[]);

    attack(&mut game, otharri);

    assert_eq!(experience(&game), 1, "one experience counter");
    let made = rebels(&game);
    assert_eq!(made.len(), 1, "and one Rebel for it");
    assert_eq!(game.power(made[0]), Some(2));
    assert_eq!(game.toughness(made[0]), Some(2));
    assert!(made[0].tapped, "tapped");
    assert!(made[0].attacking, "and attacking");
}

/// The counters are the player's and they add up: a second attack makes two
/// more Rebels.
#[test]
fn the_counters_add_up_across_attacks() {
    let (mut game, otharri, _) = staged(&[]);
    attack(&mut game, otharri);

    // Round to his next attack with everything untapped again.
    game.turns_started = [3, 2];
    for permanent in &mut game.battlefield {
        permanent.tapped = false;
        permanent.attacking = false;
    }
    game.attackers_declared = false;
    attack(&mut game, otharri);

    assert_eq!(experience(&game), 2, "a second counter");
    assert_eq!(
        rebels(&game).len(),
        3,
        "one Rebel from the first attack and two from the second",
    );
}

/// The counters belong to the player, so they outlive him.
#[test]
fn the_counters_survive_him() {
    let (mut game, otharri, _) = staged(&[]);
    attack(&mut game, otharri);

    game.move_permanents_to_graveyard(&[otharri]);
    drain_pending(&mut game);

    assert_eq!(experience(&game), 1, "the counter is the player's");
}

/// From the graveyard, tapping a Rebel brings him back tapped.
#[test]
fn a_rebel_taps_to_bring_him_back() {
    let (mut game, otharri, _) = staged(&[]);
    attack(&mut game, otharri);
    let rebel = rebels(&game)[0].card.id;
    game.move_permanents_to_graveyard(&[otharri]);
    drain_pending(&mut game);
    // The Rebel came in tapped and attacking; untap it so it can pay.
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == rebel)
        .expect("it is here")
        .tapped = false;
    let returning = game.players[0]
        .graveyard
        .iter()
        .find(|card| card.definition == cards::OTHARRI_SUNS_GLORY)
        .expect("he is in the graveyard")
        .id;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == returning))
        .expect("four mana and an untapped Rebel buy him back");
    game.apply(PlayerId::One, action).expect("it activates");
    drain_pending(&mut game);

    let back = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::OTHARRI_SUNS_GLORY)
        .expect("he is back");
    assert!(back.tapped, "and he comes back tapped");
    assert!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == rebel)
            .is_some_and(|permanent| permanent.tapped),
        "the Rebel paid for it",
    );
}

/// With no untapped Rebel there is nothing to pay the cost with.
#[test]
fn without_a_rebel_he_stays_where_he_is() {
    let (mut game, otharri, _) = staged(&[]);
    game.move_permanents_to_graveyard(&[otharri]);
    drain_pending(&mut game);
    let returning = game.players[0]
        .graveyard
        .iter()
        .find(|card| card.definition == cards::OTHARRI_SUNS_GLORY)
        .expect("he is in the graveyard")
        .id;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);

    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == returning)),
        "the mana alone does not buy him back",
    );
}
