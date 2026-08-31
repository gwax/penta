//! Leyline Binding: six mana on paper and one in a deck with every basic
//! land type, cast at instant speed.

use super::*;

/// Player One holding the Binding, with `lands` on the battlefield.
fn staged(lands: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    for definition in lands {
        game.put_onto_battlefield(PlayerId::One, *definition)
            .expect("cataloged");
    }
    // Tapped, so what is castable is decided by the mana in the pool rather
    // than by what the lands could still make. Domain counts them either way.
    for permanent in &mut game.battlefield {
        permanent.tapped = true;
    }
    drain_pending(&mut game);
    let binding = game
        .build_zone(PlayerId::One, &[cards::LEYLINE_BINDING])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = binding.id;
    game.players[0].hand.push(binding);
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, id)
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

fn castable(game: &Game, card: GameObjectId) -> bool {
    game.legal_actions(PlayerId::One)
        .iter()
        .any(|action| matches!(action, Action::CastSpell { card: id, .. } if *id == card))
}

/// Every basic land type shaves five off the cost.
#[test]
fn domain_pays_for_five_of_the_six() {
    let (mut game, binding) = staged(&[
        cards::PLAINS,
        cards::ISLAND,
        cards::SWAMP,
        cards::MOUNTAIN,
        cards::FOREST,
    ]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);

    assert!(
        castable(&game, binding),
        "five basic land types leave a single white",
    );
}

/// Two types leaves four to pay.
#[test]
fn fewer_types_leave_more_to_pay() {
    let (mut game, binding) = staged(&[cards::PLAINS, cards::ISLAND]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 3);
    assert!(!castable(&game, binding), "three mana is not enough");

    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    assert!(castable(&game, binding), "four is");
}

/// It has flash, so it answers something on their turn.
#[test]
fn it_can_be_cast_on_their_turn() {
    let (mut game, binding) = staged(&[
        cards::PLAINS,
        cards::ISLAND,
        cards::SWAMP,
        cards::MOUNTAIN,
        cards::FOREST,
    ]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    game.active_player = PlayerId::Two;
    game.step = Step::End;
    game.priority = PlayerId::One;

    assert!(castable(&game, binding));
}

/// It exiles a nonland permanent they control, and gives it back when the
/// enchantment goes.
#[test]
fn it_holds_a_permanent_until_it_leaves() {
    let (mut game, binding) = staged(&[
        cards::PLAINS,
        cards::ISLAND,
        cards::SWAMP,
        cards::MOUNTAIN,
        cards::FOREST,
    ]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    let bears = creature(220_000, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == binding))
        .expect("one white pays for it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == bears_id),
        "their creature is under the enchantment",
    );

    let enchantment = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::LEYLINE_BINDING)
        .expect("it resolved")
        .card
        .id;
    game.destroy_permanent(enchantment);
    settle(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::GRIZZLY_BEARS),
        "and it comes back when the enchantment goes",
    );
}

/// A land they control is not a legal target.
#[test]
fn a_land_is_not_a_legal_target() {
    let (mut game, binding) = staged(&[
        cards::PLAINS,
        cards::ISLAND,
        cards::SWAMP,
        cards::MOUNTAIN,
        cards::FOREST,
    ]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    game.put_onto_battlefield(PlayerId::Two, cards::MOUNTAIN)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == binding))
        .expect("one white pays for it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game);

    assert_eq!(
        game.battlefield
            .iter()
            .filter(|permanent| permanent.card.definition == cards::MOUNTAIN)
            .count(),
        2,
        "their land is still there: the trigger found nothing to name",
    );
}

/// "Leyline Binding's domain ability doesn't change its mana value, which is
/// always 6." Five basic land types buy it for one white mana, and the spell
/// on the stack is still a six-drop.
#[test]
fn the_discount_does_not_change_what_it_is_worth() {
    let (mut game, binding) = staged(&[
        cards::PLAINS,
        cards::ISLAND,
        cards::SWAMP,
        cards::MOUNTAIN,
        cards::FOREST,
    ]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    game.battlefield
        .push(creature(220_200, cards::GRIZZLY_BEARS, PlayerId::Two));

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == binding))
        .expect("one white pays for it");
    game.apply(PlayerId::One, cast).expect("it is cast");

    let spell = game
        .stack
        .iter()
        .find(|object| object.card.definition.card_definition() == Some(cards::LEYLINE_BINDING))
        .expect("it is on the stack");
    assert_eq!(
        game.stack_spell_mana_value(spell),
        6,
        "a discount is a discount and not a smaller card",
    );
    assert_eq!(
        game.players[0].mana_pool.white, 0,
        "and the one mana it actually cost was paid",
    );
}

/// "If a token is exiled, it ceases to exist. It won't be returned to the
/// battlefield." The Binding leaving gives back a card and has nothing to
/// give back for a token.
#[test]
fn a_token_it_exiles_never_comes_back() {
    let (mut game, binding) = staged(&[
        cards::PLAINS,
        cards::ISLAND,
        cards::SWAMP,
        cards::MOUNTAIN,
        cards::FOREST,
    ]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    let token = token_permanent(
        220_300,
        tokens::creature(&["Bird"], &[ManaColor::White], 1, 1),
        PlayerId::Two,
    );
    let token_id = token.card.id;
    game.battlefield.push(token);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == binding))
        .expect("one white pays for it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game);
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == token_id),
        "the Bird was exiled",
    );

    let enchantment = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::LEYLINE_BINDING)
        .expect("it resolved")
        .card
        .id;
    game.destroy_permanent(enchantment);
    settle(&mut game);

    assert!(
        game.battlefield.is_empty()
            || !game.battlefield.iter().any(|permanent| is_token_with(
                permanent,
                tokens::creature(&["Bird"], &[ManaColor::White], 1, 1)
            )),
        "a token that left the battlefield has nothing to come back as",
    );
}

/// "If Leyline Binding leaves the battlefield before its enters-the-
/// battlefield ability resolves, the target permanent won't be exiled."
/// One ability makes both halves, so with no Binding to give it back there
/// is nothing for the first half to take (CR 610.3b).
#[test]
fn a_binding_answered_in_response_exiles_nothing() {
    let (mut game, binding) = staged(&[
        cards::PLAINS,
        cards::ISLAND,
        cards::SWAMP,
        cards::MOUNTAIN,
        cards::FOREST,
    ]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    let bears = creature(220_100, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == binding))
        .expect("one white pays for it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    // Let the enchantment resolve and its trigger go on the stack, then
    // answer the enchantment underneath it.
    for _ in 0..8 {
        if !game.stack.is_empty()
            && game
                .battlefield
                .iter()
                .any(|permanent| permanent.card.definition == cards::LEYLINE_BINDING)
        {
            break;
        }
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .take(decision.minimum.max(1).min(decision.maximum))
                .map(|option| option.id)
                .collect::<Vec<_>>();
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the trigger names the Bears");
            continue;
        }
        let player = game.priority;
        game.apply(player, Action::PassPriority)
            .expect("the trigger goes on the stack");
    }
    let enchantment = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::LEYLINE_BINDING)
        .expect("it resolved")
        .card
        .id;
    game.destroy_permanent(enchantment);
    settle(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == bears_id),
        "the Bears were never exiled",
    );
    assert!(
        game.players[PlayerId::Two.index()].exile.is_empty(),
        "so nothing is waiting on a Binding that is not there",
    );
}
