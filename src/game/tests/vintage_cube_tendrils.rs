//! Tendrils of Agony: two life a copy, and a copy for every spell that came
//! before it.

use super::*;

/// Casts `count` cantrips for Player One so the Tendrils that follows arrives
/// with that many spells behind it this turn.
fn cast_cantrips(game: &mut Game, count: u32) {
    for index in 0..count {
        let spell = card(20_000 + index, cards::OPT, PlayerId::One);
        let spell_id = spell.id;
        game.players[PlayerId::One.index()].hand.push(spell);
        game.players[PlayerId::One.index()].mana_pool.blue = 1;
        game.priority = PlayerId::One;
        game.apply(
            PlayerId::One,
            cast_action(spell_id, Vec::new(), Vec::new(), 0),
        )
        .expect("a cantrip is castable");
        drain_pending(game);
    }
}

/// Answers every retarget offer with `keep`, then lets the stack empty. The
/// offer stands once per copy, and a copy that keeps its targets drains the
/// same player the original did.
fn settle(game: &mut Game, keep: bool) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let wanted = if keep {
                "Keep original targets"
            } else {
                "Copy with targets you"
            };
            let options = decision
                .options
                .iter()
                .find(|option| option.label == wanted)
                .map(|option| vec![option.id])
                .expect("the copy is offered both answers");
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the decision accepts what it offered");
            continue;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            return;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            return;
        }
    }
}

/// Casts a Tendrils at Player Two, leaving the stack for the caller.
fn cast_tendrils(game: &mut Game) {
    cast_tendrils_numbered(game, 10_000);
}

fn cast_tendrils_numbered(game: &mut Game, id: u32) {
    let tendrils = card(id, cards::TENDRILS_OF_AGONY, PlayerId::One);
    let tendrils_id = tendrils.id;
    game.players[PlayerId::One.index()].hand.push(tendrils);
    let pool = &mut game.players[PlayerId::One.index()].mana_pool;
    pool.black = 2;
    pool.colorless = 2;
    game.priority = PlayerId::One;
    game.apply(
        PlayerId::One,
        cast_action(
            tendrils_id,
            vec![Target::Player(PlayerId::Two)],
            Vec::new(),
            0,
        ),
    )
    .expect("four mana buys a Tendrils");
}

/// On its own it is four life swung and nothing more.
#[test]
fn a_lone_tendrils_drains_two() {
    let mut game = ready_game();
    cast_tendrils(&mut game);
    settle(&mut game, true);

    assert_eq!(game.players[1].life, 18, "two lost");
    assert_eq!(game.players[0].life, 22, "and two gained");
}

/// Storm counts what came before it, not itself: two cantrips make two
/// copies, so three drains land in total.
#[test]
fn storm_copies_it_once_per_earlier_spell() {
    let mut game = ready_game();
    cast_cantrips(&mut game, 2);
    cast_tendrils(&mut game);
    settle(&mut game, true);

    assert_eq!(
        game.players[1].life,
        20 - 6,
        "the original and its two copies",
    );
    assert_eq!(game.players[0].life, 20 + 6);
}

/// "You may choose new targets for the copies." Pointed back at yourself, a
/// copy costs you the two it gives you.
#[test]
fn a_copy_may_be_pointed_somewhere_else() {
    let mut game = ready_game();
    cast_cantrips(&mut game, 1);
    cast_tendrils(&mut game);
    settle(&mut game, false);

    assert_eq!(
        game.players[1].life, 18,
        "only the original still points across the table",
    );
    assert_eq!(
        game.players[0].life, 22,
        "the copy took two from you and gave two back",
    );
}

/// The life gained is a flat two a copy however little the target had left.
#[test]
fn the_gain_does_not_follow_what_was_actually_lost() {
    let mut game = ready_game();
    game.players[1].life = 1;
    cast_tendrils(&mut game);
    settle(&mut game, true);

    assert_eq!(game.players[1].life, -1, "life loss goes past zero");
    assert_eq!(
        game.players[0].life, 22,
        "and you still gain the printed two",
    );
}

/// Life loss is not damage, so nothing watching for damage sees it.
#[test]
fn the_loss_is_not_damage() {
    let mut game = ready_game();
    cast_tendrils(&mut game);
    settle(&mut game, true);

    assert!(
        !game
            .events
            .iter()
            .any(|event| matches!(event, GameEvent::DamageDealt { .. })),
        "a drain that dealt damage would be a different card",
    );
}

/// "The copies are put directly onto the stack. They aren't cast": a second
/// Tendrils counts the first one and the cantrip, and none of the copies the
/// first one made.
#[test]
fn the_copies_are_not_cast_and_do_not_feed_a_later_storm() {
    let mut game = ready_game();
    cast_cantrips(&mut game, 1);
    cast_tendrils(&mut game);
    settle(&mut game, true);
    assert_eq!(game.players[1].life, 16, "the original and one copy");

    cast_tendrils_numbered(&mut game, 10_001);
    settle(&mut game, true);

    assert_eq!(
        game.players[1].life,
        16 - 6,
        "two spells were cast before the second one, so it copies twice",
    );
    assert_eq!(game.players[0].life, 24 + 6);
}

/// "Spells cast from zones other than a player's hand and spells that were
/// countered are counted": the cantrip counts even though it never resolved,
/// and so does the Counterspell that answered it.
#[test]
fn a_countered_spell_still_feeds_the_storm_count() {
    let mut game = ready_game();
    let opt = card(20_100, cards::OPT, PlayerId::One);
    let opt_id = opt.id;
    game.players[PlayerId::One.index()].hand.push(opt);
    game.players[PlayerId::One.index()].mana_pool.blue = 1;
    let counterspell = card(20_101, cards::COUNTERSPELL, PlayerId::Two);
    let counterspell_id = counterspell.id;
    game.players[PlayerId::Two.index()].hand.push(counterspell);
    game.players[PlayerId::Two.index()].mana_pool.blue = 2;
    game.priority = PlayerId::One;
    game.apply(
        PlayerId::One,
        cast_action(opt_id, Vec::new(), Vec::new(), 0),
    )
    .expect("a cantrip is castable");
    acceptance_attempt_counterspell(&mut game, counterspell_id);
    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::OPT),
        "the cantrip was countered",
    );

    cast_tendrils(&mut game);
    settle(&mut game, true);

    assert_eq!(
        game.players[1].life,
        20 - 6,
        "both the countered spell and the spell that countered it count",
    );
}

/// "The triggered ability that creates the copies can itself be countered."
/// Stifle answers the storm trigger, and the spell that raised it still
/// resolves on its own.
#[test]
fn stifling_the_storm_trigger_leaves_one_drain() {
    let mut game = ready_game();
    let stifle = card(20_200, cards::STIFLE, PlayerId::Two);
    let stifle_id = stifle.id;
    game.players[PlayerId::Two.index()].hand.push(stifle);
    game.players[PlayerId::Two.index()].mana_pool.blue = 1;
    cast_cantrips(&mut game, 2);
    cast_tendrils(&mut game);

    let spell = game.stack.objects.first().expect("the Tendrils itself").id;

    let trigger = game
        .stack
        .iter()
        .map(|object| object.id)
        .find(|id| *id != spell)
        .expect("the storm trigger is above it");
    game.apply(PlayerId::One, Action::PassPriority).unwrap();
    game.apply(
        PlayerId::Two,
        cast_action(stifle_id, vec![Target::Spell(trigger)], Vec::new(), 0),
    )
    .expect("a triggered ability is what Stifle answers");
    settle(&mut game, true);

    assert_eq!(
        game.players[1].life, 18,
        "no copies were made, so only the original drained",
    );
    assert_eq!(game.players[0].life, 22);
}
