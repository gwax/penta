//! Oliphaunt: a six-mana trampler nobody casts for six mana, and the
//! Mountain it becomes for one.

use super::*;

/// Oliphaunt in hand with a library of Mountains, or on the battlefield
/// beside a bear when `deployed`.
fn staged(deployed: bool) -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::One.index()].library.clear();
    for index in 0..3 {
        game.players[PlayerId::One.index()].library.push(card(
            88_000 + index,
            cards::MOUNTAIN,
            PlayerId::One,
        ));
    }
    game.turns_started[PlayerId::One.index()] = 5;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    let bears = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    let oliphaunt = if deployed {
        game.put_onto_battlefield(PlayerId::One, cards::OLIPHAUNT)
            .expect("cataloged")
    } else {
        let instance = card(88_100, cards::OLIPHAUNT, PlayerId::One);
        let id = instance.id;
        game.players[PlayerId::One.index()].hand.push(instance);
        id
    };
    drain_pending(&mut game);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    (game, oliphaunt, bears)
}

fn permanent(game: &Game, id: GameObjectId) -> &Permanent {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("it is on the battlefield")
}

/// A 6/4 with trample.
#[test]
fn it_tramples() {
    let (game, oliphaunt, _) = staged(true);
    let beast = permanent(&game, oliphaunt);

    assert_eq!(
        (game.power(beast), game.toughness(beast)),
        (Some(6), Some(4))
    );
    assert!(game.permanent_has_executable_keyword(beast, KeywordAbility::Trample));
}

/// Attacking lends another creature two power and its trample.
#[test]
fn attacking_charges_something_else() {
    let (mut game, oliphaunt, bears) = staged(true);
    assert!(
        !game.permanent_has_executable_keyword(permanent(&game, bears), KeywordAbility::Trample)
    );

    game.step = Step::DeclareAttackers;
    game.declare_attacker(oliphaunt, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    drain_pending(&mut game);

    let charged = permanent(&game, bears);
    assert_eq!(game.power(charged), Some(4));
    assert_eq!(game.toughness(charged), Some(2), "only the power moves");
    assert!(game.permanent_has_executable_keyword(charged, KeywordAbility::Trample));
}

/// "Another": the Oliphaunt is not a legal target for its own trigger, so
/// its power is untouched.
#[test]
fn it_cannot_charge_itself() {
    let (mut game, oliphaunt, bears) = staged(true);
    game.battlefield
        .retain(|permanent| permanent.card.id != bears);

    game.step = Step::DeclareAttackers;
    game.declare_attacker(oliphaunt, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    drain_pending(&mut game);

    assert_eq!(game.power(permanent(&game, oliphaunt)), Some(6));
}

/// One mana and the card itself buys a Mountain from the library.
#[test]
fn mountaincycling_fetches_a_mountain() {
    let (mut game, oliphaunt, _) = staged(false);

    let cycle = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == oliphaunt))
        .expect("mountaincycling is offered from hand");
    game.apply(PlayerId::One, cycle).expect("it activates");
    for _ in 0..10 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .map(|option| option.id)
                .take(decision.minimum.max(1).min(decision.maximum))
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

    assert_eq!(
        game.players[PlayerId::One.index()]
            .hand
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::MOUNTAIN],
        "the Elephant is spent and a land arrives instead",
    );
    assert_eq!(game.players[PlayerId::One.index()].library.len(), 2);
    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::OLIPHAUNT),
    );
}

/// "Typecycling is a form of cycling." It is the same activated ability
/// with a search in place of the draw, so what reaches one reaches the
/// other: Zirda takes two off it, and the printed one-mana floor is what is
/// left.
#[test]
fn mountaincycling_is_cycling_for_everything_that_reads_it() {
    let (mut game, oliphaunt, _bears) = staged(false);
    game.players[PlayerId::One.index()].mana_pool = ManaPool::default();
    let cyclings = |game: &Game| {
        game.legal_actions(PlayerId::One)
            .into_iter()
            .filter(|action| {
                matches!(action, Action::ActivateAbility { source, .. } if *source == oliphaunt)
            })
            .count()
    };
    assert_eq!(cyclings(&game), 0, "with no mana it is not activatable");

    game.put_onto_battlefield(PlayerId::One, cards::ZIRDA_THE_DAWNWAKER)
        .expect("cataloged");
    drain_pending(&mut game);
    assert_eq!(
        cyclings(&game),
        0,
        "the discount does not make an ability free that prints a floor",
    );

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    assert_eq!(
        cyclings(&game),
        1,
        "one mana pays the floor, which is where {{1}} minus {{2}} stops",
    );
}

/// Answers whatever the cycling asks, taking `wanted` when it is offered,
/// and returns everything the search put on the table.
fn settle_cycling(game: &mut Game, wanted: Option<CardDefinitionId>) -> Vec<CardDefinitionId> {
    let mut offered = Vec::new();
    for _ in 0..12 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let cards = decision
                .options
                .iter()
                .filter_map(|option| {
                    option
                        .card
                        .and_then(|(_, characteristics)| characteristics.card_definition())
                })
                .collect::<Vec<_>>();
            if !cards.is_empty() {
                offered = cards;
            }
            let options = wanted
                .and_then(|wanted| {
                    decision
                        .options
                        .iter()
                        .find(|option| {
                            option
                                .card
                                .and_then(|(_, characteristics)| characteristics.card_definition())
                                == Some(wanted)
                        })
                        .map(|option| vec![option.id])
                })
                .unwrap_or_else(|| {
                    decision
                        .options
                        .iter()
                        .map(|option| option.id)
                        .take(decision.minimum.max(1).min(decision.maximum))
                        .collect()
                });
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
    offered
}

/// Cycles the Oliphaunt out of hand.
fn cycle(game: &mut Game, oliphaunt: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == oliphaunt),
        )
        .expect("mountaincycling is offered from hand");
    game.apply(PlayerId::One, action).expect("it activates");
}

/// "A Mountain card" is the land type and not the basic land: a Badlands is
/// a Swamp Mountain, and a Plains is neither.
#[test]
fn mountaincycling_finds_any_card_with_the_mountain_type() {
    let (mut game, oliphaunt, _) = staged(false);
    game.players[PlayerId::One.index()].library.clear();
    for (index, definition) in [cards::PLAINS, cards::BADLANDS, cards::MOUNTAIN]
        .into_iter()
        .enumerate()
    {
        game.players[PlayerId::One.index()].library.push(card(
            88_200 + u32::try_from(index).expect("small"),
            definition,
            PlayerId::One,
        ));
    }

    cycle(&mut game, oliphaunt);
    let mut offered = settle_cycling(&mut game, Some(cards::BADLANDS));
    offered.sort_unstable();
    let mut expected = vec![cards::BADLANDS, cards::MOUNTAIN];
    expected.sort_unstable();

    assert_eq!(
        offered, expected,
        "the dual carries the type; the Plains carries neither",
    );
    assert_eq!(
        game.players[PlayerId::One.index()]
            .hand
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::BADLANDS],
        "and the one taken is the one that comes",
    );
}

/// A search that finds nothing still costs the card: the mana and the
/// discard are the activation cost, paid before the library is looked at.
#[test]
fn cycling_with_no_mountain_still_spends_the_elephant() {
    let (mut game, oliphaunt, _) = staged(false);
    game.players[PlayerId::One.index()].library.clear();
    game.players[PlayerId::One.index()]
        .library
        .push(card(88_300, cards::PLAINS, PlayerId::One));

    cycle(&mut game, oliphaunt);
    let offered = settle_cycling(&mut game, None);

    assert!(offered.is_empty(), "a Plains is not a Mountain card");
    assert!(
        game.players[PlayerId::One.index()].hand.is_empty(),
        "nothing was found to put in hand",
    );
    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::OLIPHAUNT),
        "and the Oliphaunt was discarded to ask",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].mana_pool.total(),
        0,
        "with the mana spent either way",
    );
}
