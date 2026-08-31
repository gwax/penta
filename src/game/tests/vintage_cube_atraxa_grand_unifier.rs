//! Atraxa, Grand Unifier: ten cards face up, and one pick per card type from
//! among them.

use super::*;

/// The cards Atraxa will find, one of each type and three lands to pad the
/// ten out, with markers underneath them.
const DIG: [CardDefinitionId; 10] = [
    cards::SOL_RING,
    cards::GRIZZLY_BEARS,
    cards::CONTROL_MAGIC,
    cards::LIGHTNING_BOLT,
    cards::ISLAND,
    cards::JACE_THE_MIND_SCULPTOR,
    cards::STONE_RAIN,
    cards::ISLAND,
    cards::ISLAND,
    cards::ISLAND,
];

/// A library whose top ten are `top`, with `beneath` marker Mountains under
/// them, and an empty hand.
fn staged(top: &[CardDefinitionId], beneath: usize) -> Game {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for index in 0..beneath {
        game.players[0].library.push(card(
            98_000 + u32::try_from(index).expect("small"),
            cards::MOUNTAIN,
            PlayerId::One,
        ));
    }
    // The library reads from the back, so the dig is pushed last.
    for (index, definition) in top.iter().rev().enumerate() {
        game.players[0].library.push(card(
            98_500 + u32::try_from(index).expect("small"),
            *definition,
            PlayerId::One,
        ));
    }
    game.turns_started = [2, 1];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game
}

/// Answers every selection by taking `take`'s answer for that prompt,
/// recording each prompt and what it offered.
fn resolve_dig(
    game: &mut Game,
    mut take: impl FnMut(&str, &[CardDefinitionId]) -> bool,
) -> Vec<(String, Vec<CardDefinitionId>)> {
    let mut asked = Vec::new();
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let offered = decision
                .options
                .iter()
                .filter_map(|option| option.card.map(|(_, card)| card))
                .filter_map(ObjectCharacteristics::card_definition)
                .collect::<Vec<_>>();
            let options = if take(&decision.prompt, &offered) {
                decision
                    .options
                    .iter()
                    .map(|option| option.id)
                    .take(1)
                    .collect()
            } else {
                Vec::new()
            };
            asked.push((decision.prompt.clone(), offered));
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the offered answer is legal");
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
    asked
}

fn hand(game: &Game) -> Vec<CardDefinitionId> {
    game.players[0]
        .hand
        .iter()
        .map(|card| card.definition)
        .collect()
}

/// Each type is asked about once, in turn, and each pick comes home.
#[test]
fn it_takes_one_card_of_each_type() {
    let mut game = staged(&DIG, 5);

    game.put_onto_battlefield(PlayerId::One, cards::ATRAXA_GRAND_UNIFIER)
        .expect("cataloged");
    let asked = resolve_dig(&mut game, |_, _| true);

    assert_eq!(
        asked
            .iter()
            .map(|(prompt, _)| prompt.as_str())
            .collect::<Vec<_>>(),
        [
            "Put an artifact card from among them into your hand",
            "Put a creature card from among them into your hand",
            "Put an enchantment card from among them into your hand",
            "Put an instant card from among them into your hand",
            "Put a land card from among them into your hand",
            "Put a planeswalker card from among them into your hand",
            "Put a sorcery card from among them into your hand",
        ],
        "one question per card type, in turn",
    );
    let mut taken = hand(&game);
    taken.sort_by_key(|definition| format!("{definition:?}"));
    let mut expected = vec![
        cards::SOL_RING,
        cards::GRIZZLY_BEARS,
        cards::CONTROL_MAGIC,
        cards::LIGHTNING_BOLT,
        cards::ISLAND,
        cards::JACE_THE_MIND_SCULPTOR,
        cards::STONE_RAIN,
    ];
    expected.sort_by_key(|definition| format!("{definition:?}"));
    assert_eq!(taken, expected, "one card of every type");
}

/// Only cards of the type being asked about are on offer.
#[test]
fn each_question_offers_only_that_type() {
    let mut game = staged(&DIG, 5);

    game.put_onto_battlefield(PlayerId::One, cards::ATRAXA_GRAND_UNIFIER)
        .expect("cataloged");
    let asked = resolve_dig(&mut game, |_, _| false);

    let offered = |kind: &str| {
        asked
            .iter()
            .find(|(prompt, _)| prompt.contains(kind))
            .map(|(_, cards)| cards.clone())
            .unwrap_or_default()
    };
    assert_eq!(offered("artifact"), vec![cards::SOL_RING]);
    assert_eq!(offered("creature"), vec![cards::GRIZZLY_BEARS]);
    assert_eq!(offered("instant"), vec![cards::LIGHTNING_BOLT]);
    assert_eq!(
        offered("land").len(),
        4,
        "every land is a candidate for the one land pick",
    );
}

/// "You may": every pick can be declined, and what was passed over goes back
/// underneath.
#[test]
fn declining_every_pick_puts_all_ten_on_the_bottom() {
    let mut game = staged(&DIG, 5);
    let library = game.players[0].library.len();

    game.put_onto_battlefield(PlayerId::One, cards::ATRAXA_GRAND_UNIFIER)
        .expect("cataloged");
    resolve_dig(&mut game, |_, _| false);

    assert!(hand(&game).is_empty(), "nothing was taken");
    assert_eq!(
        game.players[0].library.len(),
        library,
        "and every card went back",
    );
    let top = game.players[0]
        .library
        .last()
        .expect("the library is not empty");
    assert_eq!(
        top.definition,
        cards::MOUNTAIN,
        "what was underneath is on top now",
    );
}

/// One card answers one question: an artifact creature taken as the artifact
/// is no longer among the cards the creature pick sees.
#[test]
fn a_card_taken_for_one_type_is_gone_for_the_next() {
    let dig = [
        cards::ORNITHOPTER,
        cards::ISLAND,
        cards::ISLAND,
        cards::ISLAND,
        cards::ISLAND,
        cards::ISLAND,
        cards::ISLAND,
        cards::ISLAND,
        cards::ISLAND,
        cards::ISLAND,
    ];
    let mut game = staged(&dig, 5);

    game.put_onto_battlefield(PlayerId::One, cards::ATRAXA_GRAND_UNIFIER)
        .expect("cataloged");
    let asked = resolve_dig(&mut game, |prompt, _| prompt.contains("artifact"));

    assert_eq!(
        asked
            .iter()
            .find(|(prompt, _)| prompt.contains("artifact"))
            .map(|(_, offered)| offered.clone()),
        Some(vec![cards::ORNITHOPTER]),
        "the artifact creature answers the artifact question",
    );
    assert!(
        !asked.iter().any(|(prompt, _)| prompt.contains("creature")),
        "and with it gone there is no creature left to ask about",
    );
    assert_eq!(hand(&game), vec![cards::ORNITHOPTER]);
}

/// The body: 7/7 with all four keywords.
#[test]
fn she_is_a_seven_seven_with_four_keywords() {
    let mut game = staged(&DIG, 5);
    game.put_onto_battlefield(PlayerId::One, cards::ATRAXA_GRAND_UNIFIER)
        .expect("cataloged");
    resolve_dig(&mut game, |_, _| false);

    let atraxa = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::ATRAXA_GRAND_UNIFIER)
        .expect("she is on the battlefield");
    assert_eq!(game.power(atraxa), Some(7));
    assert_eq!(game.toughness(atraxa), Some(7));
    for keyword in [
        KeywordAbility::Flying,
        KeywordAbility::Vigilance,
        KeywordAbility::Deathtouch,
        KeywordAbility::Lifelink,
    ] {
        assert!(
            game.permanent_has_executable_keyword(atraxa, keyword),
            "{keyword:?}",
        );
    }
}

/// The other half of the same ruling: "If you choose it as the artifact
/// card, you could also put into your hand a creature card, and vice versa."
/// An Ornithopter answering the artifact question leaves the creature
/// question open for whatever else is in the pile.
#[test]
fn taking_an_artifact_creature_as_the_artifact_leaves_the_creature_pick() {
    let dig = [
        cards::ORNITHOPTER,
        cards::GRIZZLY_BEARS,
        cards::ISLAND,
        cards::ISLAND,
        cards::ISLAND,
        cards::ISLAND,
        cards::ISLAND,
        cards::ISLAND,
        cards::ISLAND,
        cards::ISLAND,
    ];
    let mut game = staged(&dig, 5);

    game.put_onto_battlefield(PlayerId::One, cards::ATRAXA_GRAND_UNIFIER)
        .expect("cataloged");
    let asked = resolve_dig(&mut game, |prompt, _| {
        prompt.contains("artifact") || prompt.contains("creature")
    });

    assert_eq!(
        asked
            .iter()
            .find(|(prompt, _)| prompt.contains("creature"))
            .map(|(_, offered)| offered.clone()),
        Some(vec![cards::GRIZZLY_BEARS]),
        "the Thopter is spoken for, and the Bears are still a creature card",
    );
    let mut taken = hand(&game);
    taken.sort_unstable();
    let mut both = vec![cards::ORNITHOPTER, cards::GRIZZLY_BEARS];
    both.sort_unstable();
    assert_eq!(taken, both, "so both come home");
}

/// "Reveal the top ten cards of your library" with fewer than ten there
/// reveals what there is. Four cards is four questions worth of material and
/// no shortfall to make up.
#[test]
fn a_library_shorter_than_ten_reveals_what_it_has() {
    let dig = [
        cards::SOL_RING,
        cards::GRIZZLY_BEARS,
        cards::LIGHTNING_BOLT,
        cards::ISLAND,
    ];
    let mut game = staged(&dig, 0);

    game.put_onto_battlefield(PlayerId::One, cards::ATRAXA_GRAND_UNIFIER)
        .expect("cataloged");
    let asked = resolve_dig(&mut game, |_, _| true);

    let mut taken = hand(&game);
    taken.sort_unstable();
    let mut expected = dig.to_vec();
    expected.sort_unstable();
    assert_eq!(taken, expected, "all four are one of each type");
    assert!(
        game.players[0].library.is_empty(),
        "and the library that had four cards has none left",
    );
    assert!(
        asked
            .iter()
            .all(|(_, offered)| !offered.contains(&cards::MOUNTAIN)),
        "nothing was revealed that was not there",
    );
}
