//! Cut Down: a limit read off both halves of a creature at once.

use super::*;

/// A Cut Down in hand, one black mana, and the named creature in play.
fn staged(victim: CardDefinitionId) -> (Game, CardInstanceId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    let creature = creature(86_000, victim, PlayerId::Two);
    let victim_id = creature.card.id;
    game.battlefield.push(creature);
    let cut = card(86_001, cards::CUT_DOWN, PlayerId::One);
    let cut_id = cut.id;
    game.players[0].hand.push(cut);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    (game, cut_id, victim_id)
}

fn offered_at(game: &Game, cut: CardInstanceId, victim: GameObjectId) -> bool {
    game.legal_actions(PlayerId::One).into_iter().any(|action| {
        matches!(action, Action::CastSpell { card, choices, .. }
            if card == cut
                && choices
                    .targets()
                    .iter()
                    .flat_map(crate::casting::TargetSelection::targets)
                    .any(|target| *target == Target::Permanent(victim)))
    })
}

/// A 2/2 totals four and dies.
#[test]
fn it_kills_a_creature_totalling_five_or_less() {
    let (mut game, cut, victim) = staged(cards::GRIZZLY_BEARS);
    assert!(offered_at(&game, cut, victim), "a 2/2 is in range");

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == cut))
        .expect("one black casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    pass_priority_pair(&mut game);
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != victim),
        "the creature is destroyed",
    );
}

/// A 4/4 totals eight and is not a legal target at all.
#[test]
fn a_creature_totalling_more_is_not_a_legal_target() {
    let (game, cut, victim) = staged(cards::SERRA_ANGEL);

    assert!(!offered_at(&game, cut, victim), "eight is more than five");
}

/// Pumps the creature and reports its new stats.
fn pump(game: &mut Game, victim: GameObjectId, power: i32, toughness: i32) {
    attach_constant_resolved_characteristics(
        game,
        victim,
        &[AppliedEffectDef::modify_power_toughness(
            ValueDef::Constant(power),
            ValueDef::Constant(toughness),
        )],
        ContinuousEffectExpiration::Never,
    );
}

/// Exactly five is in range: the limit is inclusive. A 2/2 pumped to a 3/2
/// totals five and is still a legal target.
#[test]
fn the_limit_is_inclusive() {
    let (mut game, cut, victim) = staged(cards::GRIZZLY_BEARS);

    pump(&mut game, victim, 1, 0);

    assert!(offered_at(&game, cut, victim), "three and two is five");
}

/// Read live: pumping a creature past the sum takes it out of range.
#[test]
fn pumping_it_out_of_range_takes_the_target_away() {
    let (mut game, cut, victim) = staged(cards::GRIZZLY_BEARS);
    assert!(offered_at(&game, cut, victim), "in range to begin with");

    pump(&mut game, victim, 1, 1);

    let pumped = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == victim)
        .expect("still there");
    assert_eq!(
        (game.power(pumped), game.toughness(pumped)),
        (Some(3), Some(3)),
        "the pump landed",
    );
    assert!(
        !offered_at(&game, cut, victim),
        "three and three is six, and out of reach",
    );
}

/// The same rule enforced on the way down: a creature that grows while the
/// spell is on the stack is no longer a legal target, so nothing is
/// destroyed and the spell is countered on resolution.
#[test]
fn growing_in_response_leaves_it_with_nothing_to_destroy() {
    let (mut game, cut, victim) = staged(cards::GRIZZLY_BEARS);
    game.apply(
        PlayerId::One,
        cast_action(cut, vec![Target::Permanent(victim)], Vec::new(), 0),
    )
    .expect("a 2/2 is a legal target when it is cast");

    pump(&mut game, victim, 1, 1);
    pass_priority_pair(&mut game);
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == victim),
        "a 3/3 totals six and the spell cannot touch it",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::CUT_DOWN),
        "the spell had no legal target left and was countered",
    );
}

/// "The total power and toughness of a creature is determined by adding its
/// power and toughness." Adding, not counting: a creature whose power has
/// been pushed below zero brings its own total down, and a Serra Angel out
/// of range at 4/4 is in range at -2/6.
#[test]
fn a_negative_power_lowers_the_total() {
    let (mut game, cut, victim) = staged(cards::SERRA_ANGEL);
    assert!(!offered_at(&game, cut, victim), "four and four is eight");

    pump(&mut game, victim, -6, 2);

    let shrunk = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == victim)
        .expect("six toughness keeps it alive");
    assert_eq!(
        (game.power(shrunk), game.toughness(shrunk)),
        (Some(-2), Some(6)),
        "power goes below zero rather than stopping there",
    );
    assert!(
        offered_at(&game, cut, victim),
        "minus two and six is four, which is in range",
    );
}

/// "Target creature" does not say whose. Nothing stops it from answering
/// your own.
#[test]
fn your_own_creature_is_a_legal_target() {
    let (mut game, cut, _victim) = staged(cards::GRIZZLY_BEARS);
    let mine = creature(86_002, cards::GRIZZLY_BEARS, PlayerId::One);
    let mine_id = mine.card.id;
    game.battlefield.push(mine);

    assert!(
        offered_at(&game, cut, mine_id),
        "a 2/2 of your own is in range like any other",
    );
}
