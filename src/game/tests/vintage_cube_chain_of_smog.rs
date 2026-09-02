//! Chain of Smog: two cards out of somebody's hand, and the chain handed to
//! the player who lost them.
//!
//! Where the spells file stops at the offer being made, this one follows it:
//! taken, the copy is theirs to aim, and it comes back across the table.

use super::*;

/// Both players holding `cards_each`, with Player One holding a Chain and
/// the mana for it.
fn staged(cards_each: usize) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    for player in [PlayerId::One, PlayerId::Two] {
        game.players[player.index()].hand.clear();
        game.players[player.index()].graveyard.clear();
        for index in 0..cards_each {
            game.players[player.index()].hand.push(card(
                99_000
                    + u32::try_from(index).expect("a few cards")
                    + 20 * u32::from(player == PlayerId::Two),
                cards::LIGHTNING_BOLT,
                player,
            ));
        }
    }
    let chain = card(99_100, cards::CHAIN_OF_SMOG, PlayerId::One);
    let chain_id = chain.id;
    game.players[PlayerId::One.index()].hand.push(chain);
    game.players[PlayerId::One.index()].mana_pool.black = 1;
    game.players[PlayerId::One.index()].mana_pool.colorless = 1;
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, chain_id)
}

fn cast_at(game: &mut Game, chain: GameObjectId, target: PlayerId) {
    game.apply(
        PlayerId::One,
        cast_action(chain, vec![Target::Player(target)], Vec::new(), 0),
    )
    .expect("any player is a legal target");
}

/// The decision waiting on somebody, with the stack pushed along until one
/// appears.
fn next_decision(game: &mut Game) -> Option<DecisionObservation> {
    for _ in 0..16 {
        if let Some(pending) = game.pending_decisions.first() {
            return Some(pending.observation.clone());
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            return None;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            return None;
        }
    }
    None
}

/// Answers `decision` with the options whose labels `wanted` names, or with
/// the first `minimum` of them when it names nothing.
fn answer(game: &mut Game, decision: &DecisionObservation, wanted: Option<&str>) {
    let options = match wanted {
        Some(label) => vec![
            decision
                .options
                .iter()
                .find(|option| option.label == label)
                .unwrap_or_else(|| panic!("{label:?} is on offer: {:?}", decision.options))
                .id,
        ],
        None => decision
            .options
            .iter()
            .map(|option| option.id)
            .take(decision.minimum)
            .collect(),
    };
    game.apply(
        decision.player,
        Action::ChooseDecision {
            decision: decision.id,
            options,
        },
    )
    .expect("the decision accepts what it offered");
}

/// Discards two, then takes the copy and points it back across the table:
/// the player who cast it is the one who loses the next two.
#[test]
fn the_copy_is_theirs_to_aim_and_it_comes_back() {
    let (mut game, chain) = staged(4);
    cast_at(&mut game, chain, PlayerId::Two);

    let discard = next_decision(&mut game).expect("they choose two to pitch");
    assert_eq!(discard.player, PlayerId::Two);
    answer(&mut game, &discard, None);

    let offer = next_decision(&mut game).expect("and are asked about the copy");
    assert_eq!(
        offer.player,
        PlayerId::Two,
        "the chain is theirs to continue"
    );
    answer(&mut game, &offer, Some("Do it"));

    let retarget = next_decision(&mut game).expect("and theirs to aim");
    answer(
        &mut game,
        &retarget,
        Some("Copy with targets your opponent"),
    );

    let theirs = next_decision(&mut game).expect("now it is the caster's turn to pitch");
    assert_eq!(
        theirs.player,
        PlayerId::One,
        "the copy was pointed back across the table",
    );
    answer(&mut game, &theirs, None);
    let back = next_decision(&mut game).expect("and the chain is offered on");
    assert_eq!(back.player, PlayerId::One);
    answer(&mut game, &back, Some("Decline"));
    drain_pending(&mut game);

    assert_eq!(
        game.players[PlayerId::Two.index()].hand.len(),
        2,
        "two out of their four",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].hand.len(),
        2,
        "and two out of the caster's, from the copy they aimed",
    );
}

/// Declining is the ordinary answer: the two cards are gone and nothing
/// follows.
#[test]
fn declining_stops_the_chain() {
    let (mut game, chain) = staged(4);
    cast_at(&mut game, chain, PlayerId::Two);

    let discard = next_decision(&mut game).expect("they choose two to pitch");
    answer(&mut game, &discard, None);
    let offer = next_decision(&mut game).expect("and are asked about the copy");
    answer(&mut game, &offer, Some("Decline"));
    drain_pending(&mut game);

    assert!(next_decision(&mut game).is_none(), "nothing else is asked");
    assert_eq!(game.players[PlayerId::Two.index()].hand.len(), 2);
    assert_eq!(
        game.players[PlayerId::One.index()].hand.len(),
        4,
        "the caster kept every card",
    );
    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::CHAIN_OF_SMOG),
        "and the Chain itself is in the graveyard",
    );
}

/// "Target player" names a player and says nothing about which: pointing it
/// at yourself is legal, and the discard is yours.
#[test]
fn it_may_name_its_own_caster() {
    let (mut game, chain) = staged(4);

    cast_at(&mut game, chain, PlayerId::One);
    let discard = next_decision(&mut game).expect("the caster chooses two to pitch");
    assert_eq!(discard.player, PlayerId::One);
    answer(&mut game, &discard, None);
    let offer = next_decision(&mut game).expect("and is asked about the copy");
    answer(&mut game, &offer, Some("Decline"));
    drain_pending(&mut game);

    assert_eq!(
        game.players[PlayerId::One.index()].hand.len(),
        2,
        "two of the caster's four",
    );
    assert_eq!(
        game.players[PlayerId::Two.index()].hand.len(),
        4,
        "and nothing of theirs",
    );
}

/// "Discards two cards" takes what there is: a player holding one loses it
/// and is still handed the chain, which is what makes an empty hand a cost
/// of nothing.
#[test]
fn a_player_holding_one_card_loses_it_and_is_still_offered_the_chain() {
    let (mut game, chain) = staged(4);
    game.players[PlayerId::Two.index()].hand.truncate(1);
    cast_at(&mut game, chain, PlayerId::Two);

    let offer = next_decision(&mut game).expect("one card is no choice, so the offer is first");
    assert_eq!(offer.player, PlayerId::Two);
    assert!(
        offer.options.iter().any(|option| option.label == "Do it"),
        "the chain is offered even with nothing left to lose: {:?}",
        offer.options,
    );
    answer(&mut game, &offer, Some("Decline"));
    drain_pending(&mut game);

    assert!(game.players[PlayerId::Two.index()].hand.is_empty());
    assert_eq!(
        game.players[PlayerId::Two.index()].graveyard.len(),
        1,
        "the one card it could take",
    );
}

/// The state the card is played for: an empty hand discards nothing and is
/// offered the copy all the same, and the copy offers another. Pointed at
/// yourself with nothing to lose, the chain runs as long as you keep saying
/// yes -- which is what turns it into a combo piece rather than a discard
/// spell.
#[test]
fn an_empty_hand_keeps_the_chain_going_for_nothing() {
    let (mut game, chain) = staged(0);
    cast_at(&mut game, chain, PlayerId::One);

    // Three links, each taken rather than declined.
    for link in 1..=3 {
        let offer = next_decision(&mut game)
            .unwrap_or_else(|| panic!("link {link} is offered even with nothing to discard"));
        assert_eq!(
            offer.player,
            PlayerId::One,
            "the chain stays with the player it was pointed at",
        );
        answer(&mut game, &offer, Some("Do it"));

        let retarget = next_decision(&mut game)
            .unwrap_or_else(|| panic!("link {link} asks where the copy points"));
        answer(&mut game, &retarget, Some("Keep original targets"));
        assert!(
            game.players[PlayerId::One.index()].hand.is_empty(),
            "nothing was lost on link {link}: there was nothing to lose",
        );
    }

    // It is still going; declining is the only thing that stops it.
    let offer = next_decision(&mut game).expect("and it is still offering");
    answer(&mut game, &offer, Some("Decline"));
    drain_pending(&mut game);

    assert!(
        game.players[PlayerId::One.index()].hand.is_empty(),
        "four resolutions and not a card lost",
    );
    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::CHAIN_OF_SMOG),
        "the one real Chain is in the graveyard; the copies were never cards",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].graveyard.len(),
        1,
        "and only the one: three copies ceased to exist rather than piling up",
    );
}
