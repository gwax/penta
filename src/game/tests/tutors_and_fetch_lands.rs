use super::*;

fn lilianas_shade_may_decision(game: &mut Game) -> DecisionObservation {
    game.put_onto_battlefield(PlayerId::One, cards::LILIANAS_SHADE)
        .expect("Liliana's Shade is cataloged");
    for _ in 0..12 {
        if let Some(decision) = game.observe(PlayerId::One).decision {
            return decision;
        }
        let player = game.priority;
        game.apply(player, Action::PassPriority).unwrap();
    }
    panic!("Liliana's Shade never offered its optional search");
}

#[test]
fn enlightened_tutor_filters_reveals_and_puts_the_same_object_on_top() {
    let mut game = ready_game();
    game.players[0].library.clear();
    game.players[0].library.extend([
        card(13_000, cards::SERRA_ANGEL, PlayerId::One),
        card(13_001, cards::CRUSADE, PlayerId::One),
        card(13_002, cards::BLACK_LOTUS, PlayerId::One),
    ]);
    let tutor = card(13_100, cards::ENLIGHTENED_TUTOR, PlayerId::One);
    game.players[0].hand.push(tutor.clone());
    game.players[0].mana_pool.white = 1;
    let event_start = game.events().len();

    game.apply(
        PlayerId::One,
        cast_action(tutor.id, Vec::new(), Vec::new(), 0),
    )
    .unwrap();
    pass_priority_pair(&mut game);

    let decision = game.observe(PlayerId::One).decision.unwrap();
    assert_eq!(decision.visibility, DecisionVisibility::Private);
    assert_eq!((decision.minimum, decision.maximum), (0, 1));
    assert!(game.observe(PlayerId::Two).decision.is_none());
    let offered = decision
        .options
        .iter()
        .filter_map(|option| {
            option
                .card
                .and_then(|(_, characteristics)| characteristics.card_definition())
        })
        .collect::<Vec<_>>();
    assert!(offered.contains(&cards::BLACK_LOTUS));
    assert!(offered.contains(&cards::CRUSADE));
    assert!(!offered.contains(&cards::SERRA_ANGEL));
    let lotus = decision
        .options
        .iter()
        .find(|option| {
            option.card
                == Some((
                    GameObjectId(13_002),
                    ObjectCharacteristics::card(cards::BLACK_LOTUS, CardPartId::PRIMARY),
                ))
        })
        .unwrap()
        .id;

    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![lotus],
        },
    )
    .unwrap();

    let top = game.players[0].library.last().unwrap();
    assert_eq!(
        (top.id, top.definition),
        (GameObjectId(13_002), cards::BLACK_LOTUS)
    );
    assert!(game.events()[event_start..].iter().any(|event| {
        matches!(
            event,
            GameEvent::CardRevealed {
                player: PlayerId::One,
                card: GameObjectId(13_002),
                definition,
            } if *definition == cards::BLACK_LOTUS
        )
    }));
    assert!(game
        .events_for(PlayerId::Two)
        .iter()
        .any(|event| matches!(event, GameEvent::CardRevealed { definition, .. } if *definition == cards::BLACK_LOTUS)));
}

#[test]
fn lilianas_shade_decline_skips_the_search_reveal_and_shuffle() {
    let mut game = ready_game();
    game.players[0].library.clear();
    game.players[0].library.extend([
        card(13_110, cards::SAVANNAH_LIONS, PlayerId::One),
        card(13_111, cards::SWAMP, PlayerId::One),
        card(13_112, cards::LIGHTNING_BOLT, PlayerId::One),
        card(13_113, cards::BLACK_LOTUS, PlayerId::One),
        card(13_114, cards::SERRA_ANGEL, PlayerId::One),
    ]);
    let before = game.players[0]
        .library
        .iter()
        .map(|card| card.id)
        .collect::<Vec<_>>();
    let event_start = game.events().len();
    let may = lilianas_shade_may_decision(&mut game);
    let decline = may
        .options
        .iter()
        .find(|option| option.label == "Decline")
        .expect("the optional search can be declined")
        .id;

    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: may.id,
            options: vec![decline],
        },
    )
    .unwrap();

    assert_eq!(
        game.players[0]
            .library
            .iter()
            .map(|card| card.id)
            .collect::<Vec<_>>(),
        before,
        "declining skips the search's shuffle"
    );
    assert!(
        !game.events()[event_start..]
            .iter()
            .any(|event| matches!(event, GameEvent::CardRevealed { .. }))
    );
}

#[test]
fn lilianas_shade_acceptance_still_allows_qualified_fail_to_find() {
    let mut game = ready_game();
    game.players[0].library.clear();
    game.players[0].library.extend([
        card(13_120, cards::SAVANNAH_LIONS, PlayerId::One),
        card(13_121, cards::SWAMP, PlayerId::One),
        card(13_122, cards::LIGHTNING_BOLT, PlayerId::One),
        card(13_123, cards::BLACK_LOTUS, PlayerId::One),
        card(13_124, cards::SERRA_ANGEL, PlayerId::One),
        card(13_125, cards::CRUSADE, PlayerId::One),
    ]);
    let before = game.players[0]
        .library
        .iter()
        .map(|card| card.id)
        .collect::<Vec<_>>();
    let event_start = game.events().len();
    let may = lilianas_shade_may_decision(&mut game);
    let accept = may
        .options
        .iter()
        .find(|option| option.label != "Decline")
        .expect("the optional search can be accepted")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: may.id,
            options: vec![accept],
        },
    )
    .unwrap();

    let search = game.observe(PlayerId::One).decision.unwrap();
    assert_eq!((search.minimum, search.maximum), (0, 1));
    assert_eq!(search.options.len(), 1);
    assert_eq!(
        search.options[0]
            .card
            .and_then(|(_, characteristics)| characteristics.card_definition()),
        Some(cards::SWAMP)
    );
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: search.id,
            options: Vec::new(),
        },
    )
    .unwrap();

    assert_eq!(game.players[0].library.len(), before.len());
    assert_ne!(
        game.players[0]
            .library
            .iter()
            .map(|card| card.id)
            .collect::<Vec<_>>(),
        before,
        "accepting and failing to find still shuffles"
    );
    assert!(
        !game.events()[event_start..]
            .iter()
            .any(|event| matches!(event, GameEvent::CardRevealed { .. }))
    );
}

#[test]
fn seek_the_horizon_reveals_and_moves_three_basics_then_shuffles() {
    let mut game = ready_game();
    game.players[0].library.clear();
    game.players[0].library.extend([
        card(13_130, cards::SAVANNAH_LIONS, PlayerId::One),
        card(13_131, cards::PLAINS, PlayerId::One),
        card(13_132, cards::LIGHTNING_BOLT, PlayerId::One),
        card(13_133, cards::ISLAND, PlayerId::One),
        card(13_134, cards::BLACK_LOTUS, PlayerId::One),
        card(13_135, cards::SWAMP, PlayerId::One),
        card(13_136, cards::SERRA_ANGEL, PlayerId::One),
        card(13_137, cards::CRUSADE, PlayerId::One),
    ]);
    let selected_ids = [
        GameObjectId(13_131),
        GameObjectId(13_133),
        GameObjectId(13_135),
    ];
    let remainder_before = game.players[0]
        .library
        .iter()
        .filter(|card| !selected_ids.contains(&card.id))
        .map(|card| card.id)
        .collect::<Vec<_>>();
    let seek = card(13_140, cards::SEEK_THE_HORIZON, PlayerId::One);
    game.players[0].hand.push(seek.clone());
    game.players[0].mana_pool.green = 1;
    game.players[0].mana_pool.colorless = 3;
    let event_start = game.events().len();

    game.apply(
        PlayerId::One,
        cast_action(seek.id, Vec::new(), Vec::new(), 0),
    )
    .unwrap();
    pass_priority_pair(&mut game);

    let search = game.observe(PlayerId::One).decision.unwrap();
    assert_eq!((search.minimum, search.maximum), (0, 3));
    assert_eq!(search.options.len(), 3);
    let choices = search.options.iter().map(|option| option.id).collect();
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: search.id,
            options: choices,
        },
    )
    .unwrap();

    let hand = game.players[0]
        .hand
        .iter()
        .map(|card| card.definition)
        .collect::<Vec<_>>();
    for basic in [cards::PLAINS, cards::ISLAND, cards::SWAMP] {
        assert!(hand.contains(&basic), "the selected basic moved to hand");
        assert!(game.events()[event_start..].iter().any(|event| {
            matches!(event, GameEvent::CardRevealed { definition, .. } if *definition == basic)
        }));
    }
    let remainder_after = game.players[0]
        .library
        .iter()
        .map(|card| card.id)
        .collect::<Vec<_>>();
    assert_eq!(remainder_after.len(), remainder_before.len());
    assert_ne!(
        remainder_after, remainder_before,
        "the remaining library shuffled"
    );
}

#[test]
fn each_onslaught_fetch_land_pays_its_cost_and_finds_the_named_land_types() {
    let cases = [
        (cards::BLOODSTAINED_MIRE, cards::BADLANDS, cards::TUNDRA),
        (cards::FLOODED_STRAND, cards::TUNDRA, cards::BADLANDS),
        (cards::POLLUTED_DELTA, cards::UNDERGROUND_SEA, cards::TAIGA),
        (
            cards::WINDSWEPT_HEATH,
            cards::SAVANNAH,
            cards::VOLCANIC_ISLAND,
        ),
        (cards::WOODED_FOOTHILLS, cards::TAIGA, cards::TUNDRA),
    ];

    for (fetch, matching, off_pair) in cases {
        let mut game = ready_game();
        let source = game.put_onto_battlefield(PlayerId::One, fetch).unwrap();
        game.players[0].library.clear();
        game.players[0].library.extend([
            card(13_200, off_pair, PlayerId::One),
            card(13_201, matching, PlayerId::One),
        ]);
        let event_start = game.events().len();
        let action = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| {
                matches!(action, Action::ActivateAbility { source: actual, .. } if *actual == source)
            })
            .unwrap_or_else(|| panic!("fetch ability was not offered for {fetch:?}"));

        game.apply(PlayerId::One, action).unwrap();

        assert_eq!(game.players[0].life, 19);
        assert!(
            game.battlefield
                .iter()
                .all(|permanent| permanent.card.id != source)
        );
        assert!(
            game.players[0]
                .graveyard
                .iter()
                .any(|card| card.definition == fetch)
        );
        assert!(game.events()[event_start..].iter().any(|event| {
            matches!(
                event,
                GameEvent::LifeLost {
                    player: PlayerId::One,
                    amount: 1,
                }
            )
        }));
        assert_eq!(game.stack.len(), 1, "costs are paid before resolution");

        pass_priority_pair(&mut game);
        let decision = game.observe(PlayerId::One).decision.unwrap();
        assert_eq!(decision.visibility, DecisionVisibility::Private);
        assert_eq!((decision.minimum, decision.maximum), (0, 1));
        assert_eq!(
            decision
                .options
                .iter()
                .filter_map(|option| option
                    .card
                    .and_then(|(_, characteristics)| characteristics.card_definition()))
                .collect::<Vec<_>>(),
            vec![matching]
        );
        let option = decision.options[0].id;
        game.apply(
            PlayerId::One,
            Action::ChooseDecision {
                decision: decision.id,
                options: vec![option],
            },
        )
        .unwrap();

        assert!(game.battlefield.iter().any(|permanent| {
            permanent.controller == PlayerId::One && permanent.card.definition == matching
        }));
        assert_eq!(game.players[0].library.len(), 1);
        assert_eq!(game.players[0].library[0].definition, off_pair);
    }
}

#[test]
fn each_zendikar_fetch_land_finds_its_enemy_pair_and_not_the_other_one() {
    // The five enemy pairs, each with a dual that carries both named types
    // and one that carries neither.
    let cases = [
        (cards::ARID_MESA, cards::PLATEAU, cards::TROPICAL_ISLAND),
        (cards::MARSH_FLATS, cards::SCRUBLAND, cards::TAIGA),
        (
            cards::MISTY_RAINFOREST,
            cards::TROPICAL_ISLAND,
            cards::BADLANDS,
        ),
        (
            cards::SCALDING_TARN,
            cards::VOLCANIC_ISLAND,
            cards::SAVANNAH,
        ),
        (cards::VERDANT_CATACOMBS, cards::BAYOU, cards::TUNDRA),
    ];

    for (fetch, matching, off_pair) in cases {
        let mut game = ready_game();
        let source = game.put_onto_battlefield(PlayerId::One, fetch).unwrap();
        game.players[0].library.clear();
        game.players[0].library.extend([
            card(13_400, off_pair, PlayerId::One),
            card(13_401, matching, PlayerId::One),
        ]);
        let action = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| {
                matches!(action, Action::ActivateAbility { source: actual, .. } if *actual == source)
            })
            .unwrap_or_else(|| panic!("fetch ability was not offered for {fetch:?}"));

        game.apply(PlayerId::One, action).unwrap();
        assert_eq!(game.players[0].life, 19, "the life is paid as a cost");

        pass_priority_pair(&mut game);
        let decision = game.observe(PlayerId::One).decision.unwrap();
        assert_eq!(
            decision
                .options
                .iter()
                .filter_map(|option| option
                    .card
                    .and_then(|(_, characteristics)| characteristics.card_definition()))
                .collect::<Vec<_>>(),
            vec![matching],
            "only the land carrying both named types is offered"
        );
        game.apply(
            PlayerId::One,
            Action::ChooseDecision {
                decision: decision.id,
                options: vec![decision.options[0].id],
            },
        )
        .unwrap();

        assert!(game.battlefield.iter().any(|permanent| {
            permanent.controller == PlayerId::One && permanent.card.definition == matching
        }));
        assert_eq!(game.players[0].library.len(), 1);
        assert_eq!(game.players[0].library[0].definition, off_pair);
    }
}

#[test]
fn a_fetch_finishes_the_lands_as_enters_choice_before_shuffling() {
    let mut game = ready_game();
    let source = game
        .put_onto_battlefield(PlayerId::One, cards::FLOODED_STRAND)
        .unwrap();
    game.players[0].library = (13_300..13_312)
        .map(|id| card(id, cards::MOUNTAIN, PlayerId::One))
        .chain(std::iter::once(card(
            13_312,
            cards::HALLOWED_FOUNTAIN,
            PlayerId::One,
        )))
        .collect();
    let remaining_before = game.players[0].library[..12]
        .iter()
        .map(|card| card.id)
        .collect::<Vec<_>>();
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { source: actual, .. } if *actual == source)
        })
        .expect("Flooded Strand's ability is offered");

    game.apply(PlayerId::One, action).unwrap();
    pass_priority_pair(&mut game);
    let search = game.observe(PlayerId::One).decision.unwrap();
    let fountain = search
        .options
        .iter()
        .find(|option| {
            option.card
                == Some((
                    GameObjectId(13_312),
                    ObjectCharacteristics::card(cards::HALLOWED_FOUNTAIN, CardPartId::PRIMARY),
                ))
        })
        .expect("Flooded Strand finds a Plains Island")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: search.id,
            options: vec![fountain],
        },
    )
    .unwrap();

    let entry = game.observe(PlayerId::One).decision.unwrap();
    assert!(entry.prompt.contains("Hallowed Fountain"));
    assert_eq!(
        game.players[0]
            .library
            .iter()
            .map(|card| card.id)
            .collect::<Vec<_>>(),
        remaining_before,
        "the search has not shuffled while the land is still entering"
    );
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: entry.id,
            options: vec![0],
        },
    )
    .unwrap();

    let fountain = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::HALLOWED_FOUNTAIN)
        .expect("Hallowed Fountain finished entering");
    assert!(fountain.tapped, "declining the life payment taps the land");
    let remaining_after = game.players[0]
        .library
        .iter()
        .map(|card| card.id)
        .collect::<Vec<_>>();
    assert_eq!(remaining_after.len(), remaining_before.len());
    assert_ne!(
        remaining_after, remaining_before,
        "the search shuffles immediately after entry finishes"
    );
    assert!(game.pending_procedures.is_empty());
}

/// The life and the sacrifice are costs, so they are paid whatever the
/// search turns up: a fetch may decline what it found, and a library with
/// nothing to find leaves it just as poor. Either way the land is gone and
/// the life with it.
#[test]
fn a_fetch_pays_its_cost_even_when_it_finds_nothing() {
    // Declining a legal card, and having none to decline.
    for library in [
        vec![cards::TROPICAL_ISLAND, cards::MOUNTAIN],
        vec![cards::MOUNTAIN, cards::MOUNTAIN],
    ] {
        let mut game = ready_game();
        game.battlefield.clear();
        let source = game
            .put_onto_battlefield(PlayerId::One, cards::MISTY_RAINFOREST)
            .expect("cataloged");
        game.players[0].library.clear();
        for (index, definition) in library.iter().enumerate() {
            game.players[0].library.push(card(
                13_600 + u32::try_from(index).expect("two cards"),
                *definition,
                PlayerId::One,
            ));
        }
        let life = game.players[0].life;
        let deck = game.players[0].library.len();

        let action = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| {
                matches!(action, Action::ActivateAbility { source: actual, .. } if *actual == source)
            })
            .expect("the fetch is offered");
        game.apply(PlayerId::One, action).expect("it activates");
        assert_eq!(game.players[0].life, life - 1, "the life is a cost");
        assert!(
            game.battlefield
                .iter()
                .all(|permanent| permanent.card.id != source),
            "and so is the land itself",
        );

        pass_priority_pair(&mut game);
        if let Some(decision) = game.observe(PlayerId::One).decision {
            game.apply(
                PlayerId::One,
                Action::ChooseDecision {
                    decision: decision.id,
                    options: Vec::new(),
                },
            )
            .expect("taking nothing is allowed");
        }
        drain_pending(&mut game);

        assert_eq!(
            game.players[0].library.len(),
            deck,
            "every card stayed in the library",
        );
        assert!(
            !game
                .battlefield
                .iter()
                .any(|permanent| permanent.controller == PlayerId::One),
            "and nothing arrived to replace what was sacrificed",
        );
    }
}

/// Stifling a fetch is the oldest use of the card: the life and the land
/// are costs and are already spent when the ability goes on the stack, so
/// countering it leaves its controller a land down, a life down, and with
/// nothing to show for either.
#[test]
fn a_stifled_fetch_has_already_paid_for_itself() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[1].hand.clear();
    let source = game
        .put_onto_battlefield(PlayerId::One, cards::MISTY_RAINFOREST)
        .expect("cataloged");
    game.players[0].library.clear();
    game.players[0]
        .library
        .push(card(13_700, cards::TROPICAL_ISLAND, PlayerId::One));
    let stifle = card(13_701, cards::STIFLE, PlayerId::Two);
    let stifle_id = stifle.id;
    game.players[1].hand.push(stifle);
    game.players[1].mana_pool.blue = 1;
    let life = game.players[0].life;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { source: actual, .. } if *actual == source)
        })
        .expect("the fetch is offered");
    game.apply(PlayerId::One, action).expect("it activates");
    let ability = game.stack.last().expect("the search is on the stack").id;
    assert_eq!(game.players[0].life, life - 1, "the life went in as a cost");

    game.priority = PlayerId::Two;
    game.apply(
        PlayerId::Two,
        cast_action(stifle_id, vec![Target::Spell(ability)], Vec::new(), 0),
    )
    .expect("an activated ability is what Stifle names");
    drain_pending(&mut game);
    pass_priority_pair(&mut game);
    drain_pending(&mut game);

    assert_eq!(
        game.players[0].library.len(),
        1,
        "the search never happened",
    );
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.controller == PlayerId::One),
        "and nothing came out of it",
    );
    assert_eq!(game.players[0].life, life - 1, "the life is not given back");
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::MISTY_RAINFOREST),
        "nor is the land",
    );
}

/// "Pay 1 life" is payable at one life, and paying it is how a fetchland
/// kills its own controller.
#[test]
fn a_fetch_will_take_your_last_life() {
    let mut game = ready_game();
    game.battlefield.clear();
    let source = game
        .put_onto_battlefield(PlayerId::One, cards::MISTY_RAINFOREST)
        .expect("cataloged");
    game.players[0].library.clear();
    game.players[0]
        .library
        .push(card(13_800, cards::TROPICAL_ISLAND, PlayerId::One));
    game.players[0].life = 1;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { source: actual, .. } if *actual == source)
        })
        .expect("one life is enough to pay one life");
    game.apply(PlayerId::One, action).expect("it activates");
    game.check_state_based_actions();

    assert_eq!(game.players[0].life, 0);
    assert!(game.result.is_some(), "zero life is a loss");
}

/// "A Forest or Plains card" is read off the type line, and Dryad Arbor's
/// says Land Creature — Forest Dryad. A Heath finds it, and what arrives is
/// a creature that has just entered: it may block, but it cannot attack or
/// tap for its own mana until its controller's next turn.
#[test]
fn a_heath_can_fetch_a_dryad_arbor() {
    let mut game = ready_game();
    game.battlefield.clear();
    let source = game
        .put_onto_battlefield(PlayerId::One, cards::WINDSWEPT_HEATH)
        .expect("cataloged");
    game.players[0].library.clear();
    game.players[0]
        .library
        .push(card(13_900, cards::DRYAD_ARBOR, PlayerId::One));
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    let crack = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { source: actual, .. } if *actual == source)
        })
        .expect("the fetch is offered");
    game.apply(PlayerId::One, crack).expect("it activates");
    pass_priority_pair(&mut game);
    let decision = game
        .pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("the search asks");
    let arbor = decision
        .options
        .iter()
        .find(|option| {
            matches!(
                option.card,
                Some((_, ObjectCharacteristics::Card { definition, .. }))
                    if definition == cards::DRYAD_ARBOR
            )
        })
        .expect("a Forest card is a Forest card, creature or not")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![arbor],
        },
    )
    .expect("naming it is legal");
    drain_pending(&mut game);

    let fetched = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::DRYAD_ARBOR)
        .expect("it is on the battlefield");
    assert_eq!(game.power(fetched), Some(1), "a 1/1 as well as a land");
    assert!(
        !game.legal_actions(PlayerId::One).iter().any(|action| {
            matches!(
                action,
                Action::ActivateManaAbility { source, .. } if *source == fetched.card.id
            )
        }),
        "and it arrived this turn, so its mana ability waits (CR 302.6)",
    );
}

/// Putting a land onto the battlefield is not playing one (CR 305.1), so
/// the land drop is still there afterwards.
#[test]
fn a_fetch_does_not_spend_the_land_drop() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let source = game
        .put_onto_battlefield(PlayerId::One, cards::WINDSWEPT_HEATH)
        .expect("cataloged");
    game.players[0].library.clear();
    game.players[0]
        .library
        .push(card(13_910, cards::SAVANNAH, PlayerId::One));
    let held = card(13_911, cards::FOREST, PlayerId::One);
    let held_id = held.id;
    game.players[0].hand.push(held);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.players[0].lands_played_this_turn = 0;

    let crack = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { source: actual, .. } if *actual == source)
        })
        .expect("the fetch is offered");
    game.apply(PlayerId::One, crack).expect("it activates");
    pass_priority_pair(&mut game);
    drain_pending(&mut game);
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::SAVANNAH),
        "the Savannah came out",
    );

    assert!(
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::PlayLand { card, .. } if *card == held_id)),
        "and the land in hand may still be played this turn",
    );
}

/// Searching a library is done in private: the searcher sees what is there
/// and the other player is not shown the decision at all.
#[test]
fn a_library_search_is_private_to_the_searcher() {
    let mut game = ready_game();
    game.battlefield.clear();
    let source = game
        .put_onto_battlefield(PlayerId::One, cards::POLLUTED_DELTA)
        .expect("cataloged");
    game.players[0].library.clear();
    for (index, definition) in [cards::UNDERGROUND_SEA, cards::MOUNTAIN]
        .into_iter()
        .enumerate()
    {
        game.players[0].library.push(card(
            14_000 + u32::try_from(index).expect("two cards"),
            definition,
            PlayerId::One,
        ));
    }

    let crack = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { source: actual, .. } if *actual == source)
        })
        .expect("the fetch is offered");
    game.apply(PlayerId::One, crack).expect("it activates");
    pass_priority_pair(&mut game);

    let searcher = game
        .observe(PlayerId::One)
        .decision
        .expect("the searcher is asked what to find");
    assert!(
        searcher.options.iter().any(|option| matches!(
            option.card,
            Some((_, ObjectCharacteristics::Card { definition, .. }))
                if definition == cards::UNDERGROUND_SEA
        )),
        "and sees the card that qualifies",
    );
    assert!(
        game.observe(PlayerId::Two).decision.is_none(),
        "the other player is not shown a library being read",
    );
}

/// "Then shuffle" is part of the ability rather than part of finding
/// something: a search that turns up nothing still leaves the library in a
/// different order.
#[test]
fn the_library_is_shuffled_even_when_nothing_is_found() {
    let mut game = ready_game();
    game.battlefield.clear();
    let source = game
        .put_onto_battlefield(PlayerId::One, cards::POLLUTED_DELTA)
        .expect("cataloged");
    game.players[0].library.clear();
    for index in 0..8 {
        game.players[0]
            .library
            .push(card(14_100 + index, cards::MOUNTAIN, PlayerId::One));
    }
    let before = game.players[0]
        .library
        .iter()
        .map(|card| card.id)
        .collect::<Vec<_>>();

    let crack = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { source: actual, .. } if *actual == source)
        })
        .expect("the fetch is offered");
    game.apply(PlayerId::One, crack).expect("it activates");
    pass_priority_pair(&mut game);
    drain_pending(&mut game);

    let after = game.players[0]
        .library
        .iter()
        .map(|card| card.id)
        .collect::<Vec<_>>();
    assert_eq!(after.len(), before.len(), "nothing was found to take");
    assert_ne!(after, before, "but the library was shuffled all the same");
}

/// Nothing about a fetchland waits for your turn: the Catacombs are cracked
/// at the other player's end step, which is when the deck that plays them
/// would rather do it.
#[test]
fn a_fetch_may_be_cracked_on_their_turn() {
    let mut game = ready_game();
    game.battlefield.clear();
    let source = game
        .put_onto_battlefield(PlayerId::One, cards::VERDANT_CATACOMBS)
        .expect("cataloged");
    game.players[0].library.clear();
    game.players[0]
        .library
        .push(card(14_200, cards::BAYOU, PlayerId::One));
    game.active_player = PlayerId::Two;
    game.step = Step::End;
    game.priority = PlayerId::One;

    let crack = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { source: actual, .. } if *actual == source)
        })
        .expect("their end step is as good a time as any");
    game.apply(PlayerId::One, crack).expect("it activates");
    pass_priority_pair(&mut game);
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::BAYOU),
        "the Bayou is on the battlefield before your turn even begins",
    );
    assert_eq!(game.players[0].life, 19, "the life was paid on their turn");
}
