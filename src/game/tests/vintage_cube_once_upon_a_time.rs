//! Once Upon a Time: free if it is the first thing you do all game, and a
//! two-mana dig for a land or a creature ever after.

use super::*;

/// Player One holding it, with `library` stacked so the last entry is on
/// top, and no mana anywhere.
fn staged(library: &[CardDefinitionId]) -> (Game, GameObjectId) {
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
    let spell = game
        .build_zone(PlayerId::One, &[cards::ONCE_UPON_A_TIME])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let spell_id = spell.id;
    game.players[0].hand.push(spell);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, spell_id)
}

/// Five cards with one creature and one land buried in the middle of them.
const FIVE_CARDS: [CardDefinitionId; 5] = [
    cards::LIGHTNING_BOLT,
    cards::FOREST,
    cards::GRIZZLY_BEARS,
    cards::ANCESTRAL_RECALL,
    cards::COUNTERSPELL,
];

/// Answers whatever is asked, naming `wanted` if it is on offer and taking
/// nothing when it is not.
fn settle_taking(game: &mut Game, wanted: Option<CardDefinitionId>) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let mut options: Vec<_> = decision
                .options
                .iter()
                .filter(|option| {
                    wanted.is_some_and(|wanted| {
                        matches!(
                            option.card,
                            Some((_, ObjectCharacteristics::Card { definition, .. }))
                                if definition == wanted
                        )
                    })
                })
                .map(|option| option.id)
                .take(1)
                .collect();
            if options.len() < decision.minimum {
                options = decision
                    .options
                    .iter()
                    .map(|option| option.id)
                    .take(decision.minimum)
                    .collect();
            }
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

fn casts(game: &Game, spell: GameObjectId) -> Vec<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(|action| matches!(action, Action::CastSpell { card, .. } if *card == spell))
        .collect()
}

fn hand(game: &Game) -> Vec<CardDefinitionId> {
    game.players[0]
        .hand
        .iter()
        .map(|card| card.definition)
        .collect()
}

/// Nothing cast yet and no mana at all: it is still castable, and it finds
/// the creature five cards down.
#[test]
fn the_first_spell_of_the_game_is_free() {
    let (mut game, spell) = staged(&FIVE_CARDS);

    let cast = casts(&game, spell)
        .into_iter()
        .next()
        .expect("the free cast is on offer");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle_taking(&mut game, Some(cards::GRIZZLY_BEARS));

    assert_eq!(hand(&game), vec![cards::GRIZZLY_BEARS]);
    assert_eq!(
        game.players[0].library.len(),
        4,
        "the other four went to the bottom",
    );
}

/// Cast anything at all first and the free cast is gone for the rest of the
/// game -- including for the turn the spell is actually held for.
#[test]
fn a_spell_cast_earlier_closes_the_window() {
    let (mut game, spell) = staged(&FIVE_CARDS);
    let bolt = game
        .build_zone(PlayerId::One, &[cards::LIGHTNING_BOLT])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let bolt_id = bolt.id;
    game.players[0].hand.push(bolt);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    let cast = casts(&game, bolt_id)
        .into_iter()
        .next()
        .expect("the Bolt is castable");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle_taking(&mut game, None);

    assert!(
        casts(&game, spell).is_empty(),
        "one spell already cast, and no mana left for the printed cost",
    );

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 2);
    assert_eq!(
        casts(&game, spell).len(),
        1,
        "two mana still buys it the ordinary way",
    );
}

/// The window closes on the spell's own cast too: a second copy is no
/// longer free.
#[test]
fn it_is_only_free_once() {
    let (mut game, spell) = staged(&FIVE_CARDS);
    let second = game
        .build_zone(PlayerId::One, &[cards::ONCE_UPON_A_TIME])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let second_id = second.id;
    game.players[0].hand.push(second);

    let cast = casts(&game, spell)
        .into_iter()
        .next()
        .expect("the free cast is on offer");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle_taking(&mut game, Some(cards::FOREST));

    assert!(
        casts(&game, second_id).is_empty(),
        "the second copy costs mana like anything else",
    );
}

/// "You may": with nothing worth taking among the five, everything goes to
/// the bottom and the hand stays empty.
#[test]
fn a_look_with_nothing_in_it_takes_nothing() {
    let (mut game, spell) = staged(&[
        cards::LIGHTNING_BOLT,
        cards::ANCESTRAL_RECALL,
        cards::COUNTERSPELL,
        cards::LIGHTNING_BOLT,
        cards::ANCESTRAL_RECALL,
    ]);

    let cast = casts(&game, spell)
        .into_iter()
        .next()
        .expect("the free cast is on offer");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle_taking(&mut game, None);

    assert!(hand(&game).is_empty(), "no creature and no land to reveal");
    assert_eq!(game.players[0].library.len(), 5, "all five went back under");
}

/// Its ruling: "the earliest opportunity you have to cast it is during the
/// first player's upkeep, before that player can play a land." It is an
/// instant, and nothing about the free cast waits for a main phase.
#[test]
fn the_free_cast_is_available_in_an_upkeep() {
    let (mut game, spell) = staged(&FIVE_CARDS);
    game.step = Step::Upkeep;
    game.players[0].lands_played_this_turn = 0;

    let cast = casts(&game, spell)
        .into_iter()
        .next()
        .expect("an upkeep is early enough for it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle_taking(&mut game, Some(cards::FOREST));

    assert_eq!(
        hand(&game),
        vec![cards::FOREST],
        "and the land it found is in hand before the land drop",
    );
}

/// "You may reveal": with a creature and a land among the five you may still
/// take neither, and then all five go under.
#[test]
fn a_look_may_be_declined_with_something_worth_taking() {
    let (mut game, spell) = staged(&FIVE_CARDS);

    let cast = casts(&game, spell)
        .into_iter()
        .next()
        .expect("the free cast is on offer");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle_taking(&mut game, None);

    assert!(
        hand(&game).is_empty(),
        "a Bears and a Forest were there for the taking, and neither was taken",
    );
    assert_eq!(
        game.players[0].library.len(),
        5,
        "so all five went to the bottom",
    );
}

/// "You may reveal a creature or land card from among them": the other three
/// of the five are looked at and nothing more. What is offered is the whole
/// difference between this and a Sleight of Hand.
#[test]
fn only_the_creature_and_the_land_are_on_offer() {
    let (mut game, spell) = staged(&FIVE_CARDS);

    let cast = casts(&game, spell)
        .into_iter()
        .next()
        .expect("the free cast is on offer");
    game.apply(PlayerId::One, cast).expect("it is cast");
    for _ in 0..8 {
        if !game.pending_decisions.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }

    let decision = game
        .pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("the look asks which card to take");
    let mut offered = decision
        .options
        .iter()
        .filter_map(|option| match option.card {
            Some((_, ObjectCharacteristics::Card { definition, .. })) => Some(definition),
            _ => None,
        })
        .collect::<Vec<_>>();
    offered.sort_unstable();
    let mut wanted = vec![cards::FOREST, cards::GRIZZLY_BEARS];
    wanted.sort_unstable();

    assert_eq!(
        offered, wanted,
        "the Bolt, the Recall and the Counterspell were seen and not offered",
    );
}

/// "Put the rest on the bottom of your library": under whatever was already
/// down there, rather than back on top where they would be drawn again.
#[test]
fn the_rest_go_under_the_cards_that_were_already_at_the_bottom() {
    let mut library = vec![cards::MOUNTAIN; 3];
    library.extend(FIVE_CARDS);
    let (mut game, spell) = staged(&library);

    let cast = casts(&game, spell)
        .into_iter()
        .next()
        .expect("the free cast is on offer");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle_taking(&mut game, Some(cards::FOREST));

    assert_eq!(hand(&game), vec![cards::FOREST], "the land was taken");
    let library = game.players[0]
        .library
        .iter()
        .map(|card| card.definition)
        .collect::<Vec<_>>();
    assert_eq!(library.len(), 7, "one of the eight went to hand");
    assert_eq!(
        &library[4..],
        [cards::MOUNTAIN; 3],
        "the three that were never looked at are the top of the library now",
    );
    let mut under = library[..4].to_vec();
    under.sort_unstable();
    let mut rest = vec![
        cards::LIGHTNING_BOLT,
        cards::GRIZZLY_BEARS,
        cards::ANCESTRAL_RECALL,
        cards::COUNTERSPELL,
    ];
    rest.sort_unstable();
    assert_eq!(under, rest, "and the four left over are beneath them");
}

/// "The first spell *you've* cast this game": what the other player does is
/// their own business, and a Bolt from across the table leaves the free cast
/// where it was.
#[test]
fn a_spell_of_theirs_does_not_close_your_window() {
    let (mut game, spell) = staged(&FIVE_CARDS);
    game.players[1]
        .hand
        .push(card(94_800, cards::LIGHTNING_BOLT, PlayerId::Two));
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Red, 1);
    game.priority = PlayerId::Two;
    let bolt = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, .. } if *card == CardInstanceId(94_800))
        })
        .expect("their Bolt is castable");
    game.apply(PlayerId::Two, bolt).expect("it is cast");
    settle_taking(&mut game, None);
    game.priority = PlayerId::One;

    assert_eq!(
        casts(&game, spell).len(),
        1,
        "with no mana of your own, the one cast on offer is the free one",
    );
}

/// A library shorter than five is looked at as far as it goes, and what is
/// worth taking among it is still taken.
#[test]
fn a_short_library_is_looked_at_as_far_as_it_goes() {
    let (mut game, spell) = staged(&[cards::LIGHTNING_BOLT, cards::FOREST]);

    let cast = casts(&game, spell)
        .into_iter()
        .next()
        .expect("the free cast is on offer");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle_taking(&mut game, Some(cards::FOREST));

    assert_eq!(
        hand(&game),
        vec![cards::FOREST],
        "two cards were all there was, and the land among them came",
    );
    assert_eq!(
        game.players[0]
            .library
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::LIGHTNING_BOLT],
        "with the other one put underneath",
    );
}

/// "You may *reveal* a creature or land card from among them." The five are
/// looked at privately and exactly one of them is shown: the table learns
/// what you took and nothing about the four that went under.
#[test]
fn exactly_the_card_taken_is_revealed() {
    let (mut game, spell) = staged(&FIVE_CARDS);
    let cast = casts(&game, spell)
        .into_iter()
        .next()
        .expect("the free cast is on offer");
    game.apply(PlayerId::One, cast).expect("it is cast");
    game.events.clear();

    settle_taking(&mut game, Some(cards::GRIZZLY_BEARS));

    assert_eq!(
        hand(&game),
        vec![cards::GRIZZLY_BEARS],
        "the Bears was taken",
    );
    assert_eq!(
        game.events
            .iter()
            .filter(|event| matches!(event, GameEvent::CardRevealed { .. }))
            .count(),
        1,
        "one card shown, and the other four only looked at",
    );
}

/// Declining shows nothing at all: five cards looked at, none revealed, and
/// the whole look stays private.
#[test]
fn declining_reveals_nothing() {
    let (mut game, spell) = staged(&FIVE_CARDS);
    let cast = casts(&game, spell)
        .into_iter()
        .next()
        .expect("the free cast is on offer");
    game.apply(PlayerId::One, cast).expect("it is cast");
    game.events.clear();

    settle_taking(&mut game, None);

    assert!(hand(&game).is_empty(), "nothing was taken");
    assert!(
        !game
            .events
            .iter()
            .any(|event| matches!(event, GameEvent::CardRevealed { .. })),
        "and nothing was shown",
    );
}

/// Cast the ordinary way for {1}{G}, the look is the same look: the free
/// cast is a discount on the mana and nothing else.
#[test]
fn the_paid_cast_looks_at_the_same_five() {
    let (mut game, spell) = staged(&FIVE_CARDS);
    game.total_spells_cast[PlayerId::One.index()] = 1;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);

    let cast = casts(&game, spell)
        .into_iter()
        .next()
        .expect("two mana buys it the ordinary way");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle_taking(&mut game, Some(cards::FOREST));

    assert_eq!(
        hand(&game),
        vec![cards::FOREST],
        "the same five were looked at and the land came back",
    );
    assert_eq!(
        game.players[0].library.len(),
        4,
        "and the other four went under",
    );
}
