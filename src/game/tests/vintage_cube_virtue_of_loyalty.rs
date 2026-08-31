//! Virtue of Loyalty // Ardenvale Fealty: a Knight now, and the enchantment
//! later out of exile.

use super::*;

/// Player One holding the card, on their own turn, with `mana` white.
fn staged(mana: u16) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let card = game
        .build_zone(PlayerId::One, &[cards::VIRTUE_OF_LOYALTY])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let card_id = card.id;
    game.players[0].hand.push(card);
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, mana);
    (game, card_id)
}

fn settle(game: &mut Game) {
    for _ in 0..32 {
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

fn cast_with(game: &Game, card: GameObjectId, option: PlayOptionId) -> Option<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell {
                card: id, choices, ..
            } => *id == card && choices.play_option() == option,
            _ => false,
        })
}

fn tokens(game: &Game) -> Vec<GameObjectId> {
    game.battlefield
        .iter()
        .filter(|permanent| permanent.card.definition == ObjectKind::Token)
        .map(|permanent| permanent.card.id)
        .collect()
}

/// Both halves are on offer from hand, each for its own cost.
#[test]
fn both_halves_are_castable_from_hand() {
    let (game, card) = staged(5);

    assert!(
        cast_with(&game, card, PlayOptionId::DEFAULT).is_some(),
        "the enchantment is castable",
    );
    assert!(
        cast_with(&game, card, PlayOptionId(1)).is_some(),
        "and so is the Adventure",
    );
}

/// Two mana makes a 2/2 Knight with vigilance, and the card goes to exile
/// rather than to the graveyard.
#[test]
fn the_adventure_makes_a_knight_and_exiles_itself() {
    let (mut game, card) = staged(2);

    let fealty = cast_with(&game, card, PlayOptionId(1)).expect("two mana casts it");
    game.apply(PlayerId::One, fealty).expect("it is cast");
    settle(&mut game);

    let made = tokens(&game);
    assert_eq!(made.len(), 1, "one Knight");
    let knight = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == made[0])
        .expect("still there");
    assert_eq!(game.power(knight), Some(2));
    assert_eq!(game.toughness(knight), Some(2));
    assert!(game.permanent_has_executable_keyword(knight, KeywordAbility::Vigilance));

    assert!(game.players[0].graveyard.is_empty(), "not the graveyard");
    assert_eq!(
        game.players[0]
            .exile
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::VIRTUE_OF_LOYALTY],
    );
}

/// And the enchantment may be cast out of exile afterwards -- as the
/// enchantment, never as the Adventure again.
#[test]
fn the_enchantment_follows_from_exile() {
    let (mut game, card) = staged(2);
    let fealty = cast_with(&game, card, PlayOptionId(1)).expect("two mana casts it");
    game.apply(PlayerId::One, fealty).expect("it is cast");
    settle(&mut game);
    let exiled = game.players[0]
        .exile
        .first()
        .map(|card| card.id)
        .expect("the card is in exile");

    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 5);
    assert!(
        cast_with(&game, exiled, PlayOptionId(1)).is_none(),
        "the adventure cannot be taken twice",
    );
    let virtue = cast_with(&game, exiled, PlayOptionId::DEFAULT)
        .expect("the enchantment may be cast from exile");
    game.apply(PlayerId::One, virtue).expect("it is cast");
    settle(&mut game);

    assert!(
        game.battlefield.iter().any(
            |permanent| permanent.card.definition == ObjectKind::Card(cards::VIRTUE_OF_LOYALTY)
        ),
        "the enchantment is on the battlefield",
    );
}

/// The end step grows every creature you control and untaps them, and leaves
/// the other player's alone.
#[test]
fn the_end_step_grows_and_untaps_your_creatures() {
    let (mut game, card) = staged(5);
    let virtue = cast_with(&game, card, PlayOptionId::DEFAULT).expect("five mana casts it");
    game.apply(PlayerId::One, virtue).expect("it is cast");
    settle(&mut game);
    let bears = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::SERRA_ANGEL)
        .expect("cataloged");
    drain_pending(&mut game);
    game.tap_permanent(bears);
    game.tap_permanent(theirs);

    game.step = Step::End;
    game.begin_step_triggers();
    settle(&mut game);

    let grown = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == bears)
        .expect("the Bears are there");
    assert_eq!(game.power(grown), Some(3), "a counter arrived");
    assert_eq!(game.toughness(grown), Some(3));
    assert!(!grown.tapped, "and it untapped");

    let untouched = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == theirs)
        .expect("their Angel is there");
    assert_eq!(game.power(untouched), Some(4), "theirs grew nothing");
    assert!(untouched.tapped, "and stayed tapped");
}

/// It is your end step, not everybody's.
#[test]
fn it_does_nothing_on_their_end_step() {
    let (mut game, card) = staged(5);
    let virtue = cast_with(&game, card, PlayOptionId::DEFAULT).expect("five mana casts it");
    game.apply(PlayerId::One, virtue).expect("it is cast");
    settle(&mut game);
    let bears = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);

    game.active_player = PlayerId::Two;
    game.step = Step::End;
    game.begin_step_triggers();
    settle(&mut game);

    let unchanged = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == bears)
        .expect("the Bears are there");
    assert_eq!(game.power(unchanged), Some(2), "no counter on their turn");
}

/// "You must still follow any timing restrictions and permissions for the
/// permanent spell you cast from exile." Exile is a permission to cast an
/// enchantment, not a permission to cast one whenever you like.
#[test]
fn the_enchantment_in_exile_is_still_a_sorcery_speed_cast() {
    let (mut game, card) = staged(2);
    let fealty = cast_with(&game, card, PlayOptionId(1)).expect("two mana casts it");
    game.apply(PlayerId::One, fealty).expect("it is cast");
    settle(&mut game);
    let exiled = game.players[0]
        .exile
        .first()
        .map(|card| card.id)
        .expect("the card is in exile");
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 5);

    game.active_player = PlayerId::Two;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    assert!(
        cast_with(&game, exiled, PlayOptionId::DEFAULT).is_none(),
        "an enchantment is an enchantment, whoever's turn it is",
    );

    game.active_player = PlayerId::One;
    game.priority = PlayerId::One;
    assert!(
        cast_with(&game, exiled, PlayOptionId::DEFAULT).is_some(),
        "and on your own main phase it is castable again",
    );
}

/// "If an Adventure spell leaves the stack in any way other than resolving
/// ... that card won't be exiled and the spell's controller won't be able to
/// cast it as a permanent later." A countered Fealty is a card in the
/// graveyard and nothing more.
#[test]
fn a_countered_adventure_is_not_exiled_and_leaves_no_permission() {
    let (mut game, held) = staged(2);
    let fealty = cast_with(&game, held, PlayOptionId(1)).expect("two mana casts it");
    game.apply(PlayerId::One, fealty).expect("it is cast");

    let counter = card(96_000, cards::COUNTERSPELL, PlayerId::Two);
    let counter_id = counter.id;
    game.players[PlayerId::Two.index()].hand.push(counter);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Blue, 2);
    game.apply(PlayerId::One, Action::PassPriority)
        .expect("priority passes to them");
    let spell = game
        .stack
        .iter()
        .next()
        .map(|object| object.id)
        .expect("the Fealty is on the stack");
    game.apply(
        PlayerId::Two,
        cast_action(counter_id, vec![Target::Spell(spell)], Vec::new(), 0),
    )
    .expect("a spell is what Counterspell answers");
    settle(&mut game);

    assert!(
        tokens(&game).is_empty(),
        "no Knight, because the Fealty never resolved",
    );
    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::VIRTUE_OF_LOYALTY),
        "the card is in the graveyard rather than exile",
    );
    assert!(
        game.players[PlayerId::One.index()].exile.is_empty(),
        "so there is nothing in exile to cast later",
    );
}

/// "An adventurer card is a permanent card in every zone except the stack."
/// In the graveyard this is an enchantment card, whatever its other half
/// says, so a Snapcaster looking for an instant or sorcery walks past it.
#[test]
fn in_the_graveyard_it_is_an_enchantment_card() {
    let (mut game, _held) = staged(0);
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::One.index()].graveyard.clear();
    let virtue = card(96_100, cards::VIRTUE_OF_LOYALTY, PlayerId::One);
    let virtue_id = virtue.id;
    game.players[PlayerId::One.index()].graveyard.push(virtue);
    let bolt = card(96_101, cards::LIGHTNING_BOLT, PlayerId::One);
    let bolt_id = bolt.id;
    game.players[PlayerId::One.index()].graveyard.push(bolt);

    let snapcaster = card(96_102, cards::SNAPCASTER_MAGE, PlayerId::One);
    let snapcaster_id = snapcaster.id;
    game.players[PlayerId::One.index()].hand.push(snapcaster);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    game.apply(
        PlayerId::One,
        cast_action(snapcaster_id, Vec::new(), Vec::new(), 0),
    )
    .expect("it is castable");
    pass_priority_pair(&mut game);

    let offered = game
        .observe(PlayerId::One)
        .decision
        .expect("the trigger asks which card to name")
        .options
        .iter()
        .filter_map(|option| option.card.map(|(object, _)| object))
        .collect::<Vec<_>>();

    assert!(
        offered.contains(&bolt_id),
        "the Bolt is an instant card: {offered:?}",
    );
    assert!(
        !offered.contains(&virtue_id),
        "and the adventurer card is an enchantment card wherever it is not the stack",
    );
}
