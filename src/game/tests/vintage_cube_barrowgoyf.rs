//! Barrowgoyf: a body that grows with every graveyard, and a hit that digs
//! for the next creature.

use super::*;

fn settle(game: &mut Game, take: bool) {
    for _ in 0..16 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            // A "may" offers Decline as an option and insists on exactly
            // one answer; the bounded choice inside it accepts none.
            let wanted = if take {
                decision
                    .options
                    .iter()
                    .find(|option| option.label != "Decline")
            } else {
                decision
                    .options
                    .iter()
                    .find(|option| option.label == "Decline")
            };
            let options = wanted.map(|option| vec![option.id]).unwrap_or_default();
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
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
}

/// Barrowgoyf out, `graveyard` already in yours, `library` stacked to mill.
fn staged(graveyard: &[CardDefinitionId], library: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    game.players[0].library.clear();
    for (offset, definition) in graveyard.iter().enumerate() {
        let id = 80_100 + u32::try_from(offset).expect("a short graveyard");
        game.players[0]
            .graveyard
            .push(card(id, *definition, PlayerId::One));
    }
    // The top of a library is its last element, so the first card milled is
    // the last one pushed.
    for (offset, definition) in library.iter().enumerate().rev() {
        let id = 80_200 + u32::try_from(offset).expect("a short library");
        game.players[0]
            .library
            .push(card(id, *definition, PlayerId::One));
    }
    let goyf = game
        .put_onto_battlefield(PlayerId::One, cards::BARROWGOYF)
        .expect("cataloged");
    drain_pending(&mut game);
    (game, goyf)
}

fn stats(game: &Game, goyf: GameObjectId) -> (Option<i16>, Option<i16>) {
    let permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == goyf)
        .expect("the Lhurgoyf is there");
    (game.power(permanent), game.toughness(permanent))
}

/// Power counts card types across every graveyard; toughness is that plus
/// one.
#[test]
fn it_grows_with_the_card_types_in_all_graveyards() {
    let (game, goyf) = staged(&[], &[]);
    assert_eq!(stats(&game, goyf), (Some(0), Some(1)), "empty graveyards");

    let (mut game, goyf) = staged(&[cards::GRIZZLY_BEARS, cards::MOUNTAIN], &[]);
    assert_eq!(
        stats(&game, goyf),
        (Some(2), Some(3)),
        "a creature and a land is two types",
    );

    // Their graveyard counts too: "all graveyards" is not "yours".
    game.players[1]
        .graveyard
        .push(card(80_300, cards::LIGHTNING_BOLT, PlayerId::Two));
    assert_eq!(
        stats(&game, goyf),
        (Some(3), Some(4)),
        "and an instant of theirs is a third",
    );
}

/// Connecting mills that many and takes a creature from among them.
#[test]
fn combat_damage_mills_and_takes_a_creature() {
    let (mut game, goyf) = staged(
        &[cards::MOUNTAIN],
        &[cards::LIGHTNING_BOLT, cards::SERRA_ANGEL, cards::ISLAND],
    );
    // One card type in the graveyard, so the Lhurgoyf is a 1/2 and hits for
    // one... which mills only one card. Give it more to bite with.
    game.players[0]
        .graveyard
        .push(card(80_310, cards::LIGHTNING_BOLT, PlayerId::One));
    game.players[0]
        .graveyard
        .push(card(80_311, cards::BLACK_LOTUS, PlayerId::One));

    game.damage_target_from_kind(Some(goyf), Some(Target::Player(PlayerId::Two)), 3, true);
    settle(&mut game, true);
    drain_pending(&mut game);

    assert_eq!(
        game.players[0].library.len(),
        0,
        "three cards milled off a three-card library",
    );
    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::SERRA_ANGEL),
        "and the creature among them came back",
    );
}

/// Declining the mill leaves the library alone.
#[test]
fn declining_the_mill_takes_nothing() {
    let (mut game, goyf) = staged(
        &[cards::MOUNTAIN, cards::LIGHTNING_BOLT, cards::BLACK_LOTUS],
        &[cards::SERRA_ANGEL, cards::ISLAND, cards::MOUNTAIN],
    );

    game.damage_target_from_kind(Some(goyf), Some(Target::Player(PlayerId::Two)), 3, true);
    settle(&mut game, false);
    drain_pending(&mut game);

    assert_eq!(game.players[0].library.len(), 3, "nothing was milled");
    assert!(game.players[0].hand.is_empty(), "and nothing was taken");
}

/// "From among them" is the milled pile: a creature already in the
/// graveyard is not on offer.
#[test]
fn a_creature_already_in_the_graveyard_is_not_on_offer() {
    let (mut game, goyf) = staged(
        &[cards::GRIZZLY_BEARS, cards::MOUNTAIN, cards::LIGHTNING_BOLT],
        &[cards::ISLAND, cards::MOUNTAIN, cards::LIGHTNING_BOLT],
    );

    game.damage_target_from_kind(Some(goyf), Some(Target::Player(PlayerId::Two)), 3, true);
    settle(&mut game, true);
    drain_pending(&mut game);

    assert_eq!(game.players[0].library.len(), 0, "three were milled");
    assert!(
        game.players[0].hand.is_empty(),
        "no creature was among them, and the Bears in the graveyard stayed",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS),
    );
}

/// Its ruling: "the ability that defines its power and toughness works in
/// all zones, not just the battlefield." Corpse Lunge reads the power of the
/// card it exiled out of the graveyard, and the Goyf counts the graveyards
/// as they stand when it is read.
#[test]
fn its_power_is_read_out_of_the_graveyard_too() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    for (offset, definition) in [
        cards::LIGHTNING_BOLT,
        cards::PONDER,
        cards::SOL_RING,
        cards::BARROWGOYF,
    ]
    .into_iter()
    .enumerate()
    {
        game.players[0].graveyard.push(card(
            80_400 + u32::try_from(offset).expect("a short graveyard"),
            definition,
            PlayerId::One,
        ));
    }
    let wall = game
        .put_onto_battlefield(PlayerId::Two, cards::LIVING_WALL)
        .expect("cataloged");
    drain_pending(&mut game);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    let lunge = game
        .build_zone(PlayerId::One, &[cards::CORPSE_LUNGE])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let lunge_id = lunge.id;
    game.players[0].hand.push(lunge);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 3);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == lunge_id))
        .expect("the Goyf is the creature card it exiles");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game, false);

    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == wall)
            .expect("a 0/6 survives it")
            .damage,
        3,
        "an instant, a sorcery and an artifact are what is left to count",
    );
}

/// "It counts card types, not cards. If the only card in all graveyards is a
/// single artifact creature card, Barrowgoyf will be a 2/3." And a second
/// card of a type already counted adds nothing at all.
#[test]
fn one_card_of_two_types_counts_twice_and_two_of_one_counts_once() {
    let (mut game, goyf) = staged(&[cards::MYR_BATTLESPHERE], &[]);
    assert_eq!(
        stats(&game, goyf),
        (Some(2), Some(3)),
        "one artifact creature card is two types",
    );

    game.players[0]
        .graveyard
        .push(card(80_500, cards::GRIZZLY_BEARS, PlayerId::One));
    assert_eq!(
        stats(&game, goyf),
        (Some(2), Some(3)),
        "a plain creature beside it is a type already counted",
    );

    game.players[0]
        .graveyard
        .push(card(80_501, cards::SOL_RING, PlayerId::One));
    assert_eq!(
        stats(&game, goyf),
        (Some(2), Some(3)),
        "and so is a plain artifact",
    );

    game.players[0]
        .graveyard
        .push(card(80_502, cards::MOUNTAIN, PlayerId::One));
    assert_eq!(
        stats(&game, goyf),
        (Some(3), Some(4)),
        "a land is the first new type in the pile",
    );
}

/// The keywords the body is played for: what it deals is lethal to anything
/// and comes back as life.
#[test]
fn it_drains_what_it_bites() {
    let (mut game, goyf) = staged(&[cards::MOUNTAIN, cards::LIGHTNING_BOLT], &[]);
    let goyf_permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == goyf)
        .expect("he is there");
    assert!(
        game.permanent_has_executable_keyword(goyf_permanent, KeywordAbility::Deathtouch),
        "deathtouch",
    );
    assert!(
        game.permanent_has_executable_keyword(goyf_permanent, KeywordAbility::Lifelink),
        "and lifelink",
    );

    // A 2/3 into a 6/6: deathtouch makes the two lethal, lifelink makes it
    // two life.
    let titan = game
        .put_onto_battlefield(PlayerId::Two, cards::GRAVE_TITAN)
        .expect("cataloged");
    drain_pending(&mut game);
    let life = game.players[PlayerId::One.index()].life;
    game.active_player = PlayerId::One;
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.declare_attacker(goyf, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    drain_pending(&mut game);
    game.declare_blocker(titan, goyf);
    game.deal_combat_damage();
    game.check_state_based_actions();
    settle(&mut game, false);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == titan),
        "deathtouch kills a 6/6 with two damage",
    );
    assert!(
        game.players[PlayerId::One.index()].life > life,
        "and lifelink pays for the trade",
    );
}

/// "Legendary, basic, and snow are supertypes, not card types; Lhurgoyf,
/// Forest, and Siege are subtypes." A legendary creature, a basic land and
/// an Equipment carry three card types between them and not a word more.
#[test]
fn supertypes_and_subtypes_do_not_feed_it() {
    let (mut game, goyf) = staged(
        &[
            cards::ADUN_OAKENSHIELD,
            cards::FOREST,
            cards::UMEZAWAS_JITTE,
        ],
        &[],
    );
    assert_eq!(
        stats(&game, goyf),
        (Some(3), Some(4)),
        "creature, land and artifact: the legendary and the Equipment add nothing",
    );

    game.players[0]
        .graveyard
        .push(card(80_400, cards::GRIZZLY_BEARS, PlayerId::One));
    assert_eq!(
        stats(&game, goyf),
        (Some(3), Some(4)),
        "and a second creature is a type already counted",
    );

    game.players[0]
        .graveyard
        .push(card(80_401, cards::LIGHTNING_BOLT, PlayerId::One));
    assert_eq!(
        stats(&game, goyf),
        (Some(4), Some(5)),
        "while an instant is a fourth card type",
    );
}

/// Two separate "may"s: the mill is taken and the creature from among the
/// milled cards is declined, which leaves the pile in the graveyard and the
/// hand empty.
#[test]
fn the_mill_may_be_taken_and_the_creature_declined() {
    let (mut game, goyf) = staged(
        &[cards::MOUNTAIN, cards::LIGHTNING_BOLT, cards::BLACK_LOTUS],
        &[cards::SERRA_ANGEL, cards::ISLAND, cards::MOUNTAIN],
    );

    game.damage_target_from_kind(Some(goyf), Some(Target::Player(PlayerId::Two)), 3, true);

    // Yes to the mill, then no to what it turned up.
    let mut answered_first = false;
    for _ in 0..16 {
        let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        else {
            if game.stack.is_empty() && game.pending_triggers.is_empty() {
                break;
            }
            if game.apply(game.priority, Action::PassPriority).is_err() {
                break;
            }
            continue;
        };
        let wanted = if answered_first {
            decision
                .options
                .iter()
                .find(|option| option.label == "Decline")
        } else {
            decision
                .options
                .iter()
                .find(|option| option.label != "Decline")
        };
        answered_first = true;
        let options = wanted.map(|option| vec![option.id]).unwrap_or_default();
        game.apply(
            decision.player,
            Action::ChooseDecision {
                decision: decision.id,
                options,
            },
        )
        .expect("the decision accepts what it offered");
    }
    drain_pending(&mut game);

    assert!(
        game.players[0].library.is_empty(),
        "three cards were milled",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::SERRA_ANGEL),
        "the Angel is lying in the graveyard with the rest",
    );
    assert!(
        game.players[0].hand.is_empty(),
        "and declining the second may leaves it there",
    );
}

/// "You may mill that many cards": the damage goes to them and the milling
/// comes out of your own library. Which is worth pinning, because the
/// Fallen Shinobi in the same cube reads the other way and takes the cards
/// off the top of the deck it just hit.
#[test]
fn the_mill_comes_out_of_your_own_library_and_not_theirs() {
    let (mut game, goyf) = staged(
        &[cards::MOUNTAIN, cards::LIGHTNING_BOLT, cards::BLACK_LOTUS],
        &[cards::ISLAND, cards::SERRA_ANGEL, cards::GIANT_GROWTH],
    );
    game.players[1].library.clear();
    for offset in 0..5u32 {
        game.players[1]
            .library
            .push(card(80_400 + offset, cards::GRIZZLY_BEARS, PlayerId::Two));
    }
    let mine = game.players[0].library.len();

    game.damage_target_from_kind(Some(goyf), Some(Target::Player(PlayerId::Two)), 3, true);
    settle(&mut game, true);
    drain_pending(&mut game);

    assert_eq!(
        game.players[0].library.len(),
        mine - 3,
        "three off the top of your own library",
    );
    assert_eq!(
        game.players[1].library.len(),
        5,
        "and not one card off theirs, whoever took the damage",
    );
    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::SERRA_ANGEL),
        "the creature taken came out of your own three",
    );
    assert!(
        game.players[1].graveyard.is_empty(),
        "with nothing of theirs milled anywhere",
    );
}
