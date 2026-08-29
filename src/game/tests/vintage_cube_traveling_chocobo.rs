//! Traveling Chocobo: a library played face up for lands and Birds, and a
//! Panharmonicon that only reads those two.

use super::*;

/// Player One with a Chocobo out, `library` stacked so the last entry is on
/// top, and `others` also on the battlefield under them.
fn staged(library: &[CardDefinitionId], others: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for definition in library {
        let card = game
            .build_zone(PlayerId::One, &[*definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        game.players[0].library.push(card);
    }
    let chocobo = game
        .put_onto_battlefield(PlayerId::One, cards::TRAVELING_CHOCOBO)
        .expect("cataloged");
    for definition in others {
        game.put_onto_battlefield(PlayerId::One, *definition)
            .expect("cataloged");
    }
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [3, 3];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.players[0].life = 20;
    (game, chocobo)
}

fn top(game: &Game) -> GameObjectId {
    game.players[0].library.last().expect("a library").id
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

/// Puts `definition` onto the battlefield under Player One and lets whatever
/// it triggered finish.
fn arrive(game: &mut Game, definition: CardDefinitionId) {
    game.put_onto_battlefield(PlayerId::One, definition)
        .expect("cataloged");
    settle(game);
}

fn playable(game: &Game, card: GameObjectId) -> bool {
    game.legal_actions(PlayerId::One).iter().any(|action| {
        matches!(action, Action::PlayLand { card: id, .. } | Action::CastSpell { card: id, .. }
            if *id == card)
    })
}

/// The look is private: it says "you may look", not "revealed".
#[test]
fn the_top_card_is_for_its_owner_alone() {
    let (game, _chocobo) = staged(&[cards::FOREST], &[]);
    let card = top(&game);

    assert_eq!(
        game.observe(PlayerId::One).revealed_library_top,
        Some((card, cards::FOREST)),
    );
    assert_eq!(
        game.observe(PlayerId::Two).opponent_revealed_library_top,
        None,
        "nothing here plays with the card revealed",
    );
}

/// A land off the top is a land drop, and a Bird off the top is a cast.
#[test]
fn lands_and_birds_come_off_the_top() {
    let (game, _chocobo) = staged(&[cards::FOREST], &[]);
    assert!(playable(&game, top(&game)), "the land is playable");

    let (mut game, _chocobo) = staged(&[cards::BIRDS_OF_PARADISE], &[]);
    let birds = top(&game);
    assert!(
        !playable(&game, birds),
        "with no mana even a Bird stays where it is",
    );
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 1);
    assert!(playable(&game, birds), "one green casts it from up there");
}

/// Lands and Birds and nothing else: the permission names two kinds of card
/// and a Bear is neither.
#[test]
fn anything_else_stays_on_top() {
    let (mut game, _chocobo) = staged(&[cards::GRIZZLY_BEARS], &[]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 5);

    assert!(
        !playable(&game, top(&game)),
        "a creature that is not a Bird is not cast from the library",
    );
}

/// A land arriving doubles a landfall trigger, so the Courser's one life
/// becomes two.
#[test]
fn a_land_doubles_a_landfall_trigger() {
    let (mut game, _chocobo) = staged(&[], &[cards::COURSER_OF_KRUPHIX]);
    let before = game.players[0].life;

    arrive(&mut game, cards::FOREST);

    assert_eq!(
        game.players[0].life,
        before + 2,
        "the landfall trigger triggered an additional time",
    );
}

/// Each Chocobo adds one more instance rather than doubling the doubling.
#[test]
fn two_chocobos_make_it_three() {
    let (mut game, _chocobo) = staged(&[], &[cards::COURSER_OF_KRUPHIX]);
    arrive(&mut game, cards::TRAVELING_CHOCOBO);
    let before = game.players[0].life;

    arrive(&mut game, cards::FOREST);

    assert_eq!(
        game.players[0].life,
        before + 3,
        "two rules are two extra triggers, not four instances",
    );
}

/// The arrival has to be a land or a Bird. A Bear entering triggers the
/// Healer once; a Bird entering triggers it twice.
#[test]
fn only_lands_and_birds_do_the_doubling() {
    let (mut game, _chocobo) = staged(&[], &[cards::HEALER_OF_THE_PRIDE]);
    let before = game.players[0].life;

    arrive(&mut game, cards::GRIZZLY_BEARS);
    assert_eq!(
        game.players[0].life,
        before + 2,
        "a Bear is neither, so the Healer gains once",
    );

    let before = game.players[0].life;
    arrive(&mut game, cards::BIRDS_OF_PARADISE);
    assert_eq!(
        game.players[0].life,
        before + 4,
        "a Bird is one, so the Healer gains twice",
    );
}

/// "A triggered ability of a permanent you control": their Healer is not
/// doubled by your Chocobo, however Birdlike what arrived was.
#[test]
fn it_does_not_double_their_triggers() {
    let (mut game, _chocobo) = staged(&[], &[]);
    game.put_onto_battlefield(PlayerId::Two, cards::HEALER_OF_THE_PRIDE)
        .expect("cataloged");
    drain_pending(&mut game);
    let before = game.players[1].life;

    game.put_onto_battlefield(PlayerId::Two, cards::BIRDS_OF_PARADISE)
        .expect("cataloged");
    settle(&mut game);

    assert_eq!(
        game.players[1].life,
        before + 2,
        "their trigger is their own, and gains once",
    );
}

/// "Replacement effects are unaffected." Dark Depths enters with ten ice
/// counters because a replacement says so as it enters, not because
/// anything triggered: the Chocobo has nothing to add.
#[test]
fn a_replacement_on_the_way_in_is_left_alone() {
    let (mut game, _chocobo) = staged(&[], &[]);

    arrive(&mut game, cards::DARK_DEPTHS);

    let depths = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::DARK_DEPTHS)
        .expect("the land arrived");
    assert_eq!(
        depths.counters(CounterKind::named("ice")),
        10,
        "ten, not twenty: nothing triggered to be doubled",
    );
}

/// The clause reads the arriving permanent's own triggers too: a surveil
/// land that looks at one card looks at two when it lands beside a Chocobo.
#[test]
fn a_lands_own_enters_trigger_is_doubled() {
    let (mut game, _chocobo) = staged(
        &[
            cards::LIGHTNING_BOLT,
            cards::GRIZZLY_BEARS,
            cards::SERRA_ANGEL,
        ],
        &[],
    );
    let library_before = game.players[0].library.len();

    arrive(&mut game, cards::HEDGE_MAZE);

    assert_eq!(
        game.players[0].graveyard.len(),
        2,
        "two surveil triggers, and this settle bins what each one turns up",
    );
    assert_eq!(
        game.players[0].library.len(),
        library_before - 2,
        "which is two cards off the top",
    );
}
