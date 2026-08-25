//! Third Path Iconoclast: two mana for a body that turns every cantrip into
//! an artifact creature.

use super::*;

/// The Iconoclast on the battlefield since last turn, with `hand` to cast
/// and mana enough for any of it.
fn staged(hand: &[CardDefinitionId]) -> (Game, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for index in 0..10 {
        game.players[0]
            .library
            .push(card(93_000 + index, cards::ISLAND, PlayerId::One));
    }
    game.put_onto_battlefield(PlayerId::One, cards::THIRD_PATH_ICONOCLAST)
        .expect("cataloged");
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [2, 1];
    drain_pending(&mut game);
    let mut ids = Vec::new();
    for definition in hand {
        let card = game
            .build_zone(PlayerId::One, &[*definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        ids.push(card.id);
        game.players[0].hand.push(card);
    }
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 3);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 3);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 3);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, ids)
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
                .take(decision.minimum.max(1))
                .map(|option| option.id)
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

fn cast(game: &mut Game, card: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card: id, .. } if *id == card))
        .expect("it is castable");
    game.apply(PlayerId::One, action).expect("it is cast");
    settle(game);
}

fn soldiers(game: &Game) -> Vec<&Permanent> {
    game.battlefield
        .iter()
        .filter(|permanent| permanent.card.definition == ObjectKind::Token)
        .collect()
}

/// An instant makes a Soldier: a 1/1 colourless artifact creature.
#[test]
fn a_noncreature_spell_makes_a_soldier() {
    let (mut game, ids) = staged(&[cards::BRAINSTORM]);

    cast(&mut game, ids[0]);

    let made = soldiers(&game);
    assert_eq!(made.len(), 1, "one Soldier");
    assert_eq!(game.power(made[0]), Some(1));
    assert_eq!(game.toughness(made[0]), Some(1));
    assert!(
        game.permanent_types(made[0]).is_some_and(
            |types| types.contains(CardType::Artifact) && types.contains(CardType::Creature)
        ),
        "an artifact creature",
    );
}

/// A creature spell is not one of them.
#[test]
fn a_creature_spell_makes_nothing() {
    let (mut game, ids) = staged(&[cards::GRIZZLY_BEARS]);

    cast(&mut game, ids[0]);

    assert!(soldiers(&game).is_empty(), "a creature is a creature");
}

/// Every noncreature spell counts, not only instants and sorceries: an
/// artifact is one too.
#[test]
fn an_artifact_spell_counts() {
    let (mut game, ids) = staged(&[cards::SOL_RING]);

    cast(&mut game, ids[0]);

    assert_eq!(soldiers(&game).len(), 1, "the Ring is noncreature");
}

/// One Soldier per spell, so two spells make two.
#[test]
fn each_spell_makes_its_own_soldier() {
    // A Ring and a Bolt rather than anything that reshuffles the hand: what
    // is being counted is the casting, one Soldier at a time.
    let (mut game, ids) = staged(&[cards::SOL_RING, cards::LIGHTNING_BOLT]);

    cast(&mut game, ids[0]);
    let bolt = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == ids[1]
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Player(PlayerId::Two))
            }
            _ => false,
        })
        .expect("the Bolt is castable");
    game.apply(PlayerId::One, bolt).expect("it is cast");
    settle(&mut game);

    assert_eq!(soldiers(&game).len(), 2, "one for each");
}

/// Their spells are theirs: the trigger says "you cast".
#[test]
fn their_spell_makes_nothing() {
    let (mut game, _) = staged(&[]);
    let theirs = game
        .build_zone(PlayerId::Two, &[cards::LIGHTNING_BOLT])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = theirs.id;
    game.players[1].hand.push(theirs);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Red, 1);
    game.priority = PlayerId::Two;

    let action = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == id))
        .expect("they can cast it");
    game.apply(PlayerId::Two, action).expect("it is cast");
    settle(&mut game);

    assert!(soldiers(&game).is_empty(), "nothing of yours was cast");
}
