//! Mine Collapse: four mana nobody pays, and a land nobody misses.

use super::*;

/// Player One holding a Mine Collapse, with `lands` under them and a Serra
/// Angel across the table to point at.
fn staged(lands: &[CardDefinitionId]) -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    for definition in lands {
        game.put_onto_battlefield(PlayerId::One, *definition)
            .expect("cataloged");
    }
    let angel = game
        .put_onto_battlefield(PlayerId::Two, cards::SERRA_ANGEL)
        .expect("cataloged");
    let card = game
        .build_zone(PlayerId::One, &[cards::MINE_COLLAPSE])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let collapse = card.id;
    game.players[0].hand.push(card);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [1, 1];
    drain_pending(&mut game);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, collapse, angel)
}

fn settle(game: &mut Game) {
    for _ in 0..16 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
    game.check_state_based_actions();
}

/// Every way the Collapse could be cast at the Angel right now.
fn casts(game: &Game, collapse: GameObjectId, angel: GameObjectId) -> Vec<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == collapse
                    && choices
                        .targets()
                        .iter()
                        .any(|selection| selection.targets().contains(&Target::Permanent(angel)))
            }
            _ => false,
        })
        .collect()
}

/// The free half: a Mountain goes, the Angel takes five, and no mana moves.
#[test]
fn sacrificing_a_mountain_pays_for_it_on_your_turn() {
    let (mut game, collapse, angel) = staged(&[cards::MOUNTAIN]);
    let offers = casts(&game, collapse, angel);
    assert_eq!(offers.len(), 1, "only the free half is affordable");

    game.apply(PlayerId::One, offers[0].clone())
        .expect("the Mountain pays for it");
    settle(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == angel),
        "five damage killed a 4/4",
    );
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::MOUNTAIN),
        "and the Mountain was sacrificed",
    );
    assert_eq!(
        game.players[0].mana_pool.total(),
        0,
        "with no mana raised at all",
    );
}

/// "If it's your turn" gates the free half and nothing else: on their turn
/// the Collapse is a four-mana instant again.
#[test]
fn the_free_half_is_not_offered_on_their_turn() {
    let (mut game, collapse, angel) = staged(&[cards::MOUNTAIN]);
    game.active_player = PlayerId::Two;
    game.priority = PlayerId::One;

    assert!(
        casts(&game, collapse, angel).is_empty(),
        "a Mountain buys nothing on somebody else's turn",
    );

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 3);
    assert_eq!(
        casts(&game, collapse, angel).len(),
        1,
        "but four mana still casts it",
    );
}

/// The printed cost stands beside the free one when both are payable.
#[test]
fn both_halves_are_offered_when_both_are_payable() {
    let (mut game, collapse, angel) = staged(&[cards::MOUNTAIN]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 3);

    assert_eq!(
        casts(&game, collapse, angel).len(),
        2,
        "the mana cost and the sacrifice are two ways to pay one spell",
    );
}

/// A Mountain is a land type rather than a name: a dual that has it pays,
/// and a land that does not have it cannot.
#[test]
fn any_land_with_the_mountain_type_pays_it() {
    let (game, collapse, angel) = staged(&[cards::PLAINS]);
    assert!(
        casts(&game, collapse, angel).is_empty(),
        "a Plains is not a Mountain",
    );

    let (game, collapse, angel) = staged(&[cards::SACRED_FOUNDRY]);
    assert_eq!(
        casts(&game, collapse, angel).len(),
        1,
        "a Sacred Foundry is a Mountain Plains, and the Mountain half is enough",
    );
}

/// Five damage, not four: enough for most of what a red deck is afraid of.
#[test]
fn it_deals_five() {
    let (mut game, collapse, _angel) = staged(&[cards::MOUNTAIN]);
    game.create_token(
        PlayerId::Two,
        token_with_trample(tokens::creature(&["Wurm"], &[ManaColor::Green], 5, 5)),
    );
    drain_pending(&mut game);
    let wurm = game
        .battlefield
        .iter()
        .find(|permanent| {
            is_token_with(
                permanent,
                token_with_trample(tokens::creature(&["Wurm"], &[ManaColor::Green], 5, 5)),
            )
        })
        .expect("the Wurm token arrived")
        .card
        .id;
    game.priority = PlayerId::One;

    let action = casts(&game, collapse, wurm)
        .into_iter()
        .next()
        .expect("the Wurm token is a legal target");
    game.apply(PlayerId::One, action).expect("it casts");
    settle(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == wurm),
        "a 5/5 dies to it exactly",
    );
}

/// "Target creature or planeswalker": the other half of the target line.
/// Five damage to a planeswalker is five loyalty off it, and a Narset who
/// started at five is not there afterwards.
#[test]
fn it_answers_a_planeswalker_too() {
    let (mut game, collapse, _angel) = staged(&[cards::MOUNTAIN]);
    let narset = game
        .put_onto_battlefield(PlayerId::Two, cards::NARSET_PARTER_OF_VEILS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;
    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == narset)
            .expect("she arrived")
            .counters(CounterKind::Loyalty),
        5,
        "five loyalty to take five damage",
    );

    let action = casts(&game, collapse, narset)
        .into_iter()
        .next()
        .expect("a planeswalker is a legal target");
    game.apply(PlayerId::One, action).expect("it casts");
    settle(&mut game);
    game.check_state_based_actions();

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == narset),
        "damage to a planeswalker is loyalty off it, and five was all she had",
    );
}

/// CR 601.2f: a cost increase applies on top of an alternative cost rather
/// than instead of it. Thalia taxes the free half like any other noncreature
/// spell, and a Mountain already tapped can be sacrificed but cannot also
/// pay her -- which is what separates the two halves of this cost.
#[test]
fn thalia_taxes_the_free_half_as_well() {
    let (mut game, collapse, angel) = staged(&[cards::MOUNTAIN]);
    for permanent in &mut game.battlefield {
        if permanent.card.definition == cards::MOUNTAIN {
            permanent.tapped = true;
        }
    }
    assert_eq!(
        casts(&game, collapse, angel).len(),
        1,
        "a tapped Mountain is still a Mountain to sacrifice",
    );

    game.battlefield.push(creature(
        131_500,
        cards::THALIA_GUARDIAN_OF_THRABEN,
        PlayerId::Two,
    ));
    assert!(
        casts(&game, collapse, angel).is_empty(),
        "but the sacrifice does not pay the mana she asks for on top of it",
    );

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    assert_eq!(
        casts(&game, collapse, angel).len(),
        1,
        "one mana and the Mountain is the free half again",
    );
}

/// The Mountain is a cost, not an effect: it is gone as the spell is cast,
/// so a Counterspell answers the Collapse and leaves the land in the
/// graveyard all the same.
#[test]
fn the_mountain_is_spent_even_when_the_spell_is_countered() {
    let (mut game, collapse, angel) = staged(&[cards::MOUNTAIN]);
    game.players[1]
        .hand
        .push(card(85_900, cards::COUNTERSPELL, PlayerId::Two));
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Blue, 2);

    let free = casts(&game, collapse, angel)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { choices, .. } if choices.costs().alternative().is_some())
        })
        .expect("the free half is on offer");
    game.apply(PlayerId::One, free).expect("it is cast");
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::MOUNTAIN),
        "the land was sacrificed to announce it",
    );

    game.priority = PlayerId::Two;
    let counter = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, .. } if *card == CardInstanceId(85_900))
        })
        .expect("two blue answers it");
    game.apply(PlayerId::Two, counter).expect("it is cast");
    settle(&mut game);

    let angel = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == angel)
        .expect("the Angel is untouched");
    assert_eq!(angel.damage, 0, "the five never happened");
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::MOUNTAIN),
        "and the Mountain is still spent",
    );
    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.definition != cards::MOUNTAIN),
        "with nothing left on the battlefield to show for it",
    );
}

/// "Target creature or planeswalker" does not say whose: five damage is as
/// available to your own board as to theirs.
#[test]
fn it_may_be_aimed_at_your_own_creature() {
    let (mut game, collapse, _angel) = staged(&[cards::MOUNTAIN]);
    let mine = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);

    let cast = casts(&game, collapse, mine)
        .into_iter()
        .next()
        .expect("your own creature is a legal target");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game);

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != mine),
        "five damage kills your own bear too",
    );
}
