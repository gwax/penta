//! Slickshot Show-Off: a hasty flier that grows with every spell after it,
//! and a plot cost that pays for it a turn early.

use super::*;

/// Player One holding the Bird, with `hand` beside it and mana to spare.
fn staged(hand: &[CardDefinitionId]) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let card = game
        .build_zone(PlayerId::One, &[cards::SLICKSHOT_SHOW_OFF])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let held = card.id;
    game.players[0].hand.push(card);
    let mut others = Vec::new();
    for definition in hand {
        let card = game
            .build_zone(PlayerId::One, &[*definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        others.push(card.id);
        game.players[0].hand.push(card);
    }
    game.players[1].life = 20;
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, held, others)
}

fn settle(game: &mut Game) {
    for _ in 0..24 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    drain_pending(game);
}

/// Casts `card` from Player One's hand.
fn cast(game: &mut Game, card: GameObjectId) {
    for color in [ManaColor::Red, ManaColor::Colorless] {
        game.add_unrestricted_mana(PlayerId::One, color, 3);
    }
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card: id, .. } if *id == card))
        .expect("it is castable");
    game.apply(PlayerId::One, action).expect("it is cast");
    settle(game);
}

fn bird(game: &Game) -> &Permanent {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::SLICKSHOT_SHOW_OFF)
        .expect("the Bird is on the battlefield")
}

/// It arrives as a hasty 1/2 flier.
#[test]
fn it_arrives_flying_and_hasty() {
    let (mut game, held, _) = staged(&[]);

    cast(&mut game, held);

    let body = bird(&game);
    assert_eq!(game.power(body), Some(1));
    assert_eq!(game.toughness(body), Some(2));
    assert!(game.has_flying(body));
    assert!(game.permanent_has_executable_keyword(body, KeywordAbility::Haste));
}

/// Every noncreature spell after it is +2/+0, and they stack.
#[test]
fn each_noncreature_spell_grows_it() {
    let (mut game, held, others) = staged(&[cards::SHOCK, cards::SHOCK]);
    cast(&mut game, held);
    assert_eq!(game.power(bird(&game)), Some(1));

    cast(&mut game, others[0]);
    assert_eq!(game.power(bird(&game)), Some(3), "+2/+0");

    cast(&mut game, others[1]);
    assert_eq!(game.power(bird(&game)), Some(5), "and again");
    assert_eq!(game.toughness(bird(&game)), Some(2), "toughness untouched");
}

/// A creature spell is not a noncreature spell.
#[test]
fn a_creature_spell_does_nothing() {
    let (mut game, held, others) = staged(&[cards::GRIZZLY_BEARS]);
    cast(&mut game, held);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 2);

    cast(&mut game, others[0]);

    assert_eq!(game.power(bird(&game)), Some(1), "still a 1/2");
}

/// The pump is until end of turn.
#[test]
fn the_pump_wears_off() {
    let (mut game, held, others) = staged(&[cards::SHOCK]);
    cast(&mut game, held);
    cast(&mut game, others[0]);
    assert_eq!(game.power(bird(&game)), Some(3));

    let turn = game.turn;
    for _ in 0..60 {
        if game.turn > turn {
            break;
        }
        game.advance_step();
        drain_pending(&mut game);
    }

    assert_eq!(game.power(bird(&game)), Some(1), "back to a 1/2");
}

/// Plot pays the two now and casts it for nothing on a later turn.
#[test]
fn plotting_it_pays_a_turn_early() {
    let (mut game, held, _) = staged(&[]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);

    let plot = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::Plot { card } if *card == held))
        .expect("two mana pays the plot cost");
    game.apply(PlayerId::One, plot).expect("it plots");

    assert!(game.players[0].hand.is_empty());
    assert_eq!(game.players[0].exile.len(), 1, "it waits in exile");
    let plotted = game.players[0].exile[0].id;
    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == plotted)),
        "not on the turn it was plotted",
    );

    game.commit_next_turn(PlayerId::Two, Vec::new());
    game.commit_next_turn(PlayerId::One, Vec::new());
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.players[0].mana_pool = ManaPool::default();

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == plotted))
        .expect("a plotted card is castable on a later turn");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game);

    assert_eq!(game.power(bird(&game)), Some(1), "the Bird arrived");
    assert_eq!(
        game.players[0].mana_pool.total(),
        0,
        "and cost nothing to cast",
    );
}

/// "Plot only as a sorcery": the special action is available in your own
/// main phase with the stack empty and nowhere else.
#[test]
fn plotting_waits_for_your_own_empty_main_phase() {
    let (mut game, held, _) = staged(&[cards::LIGHTNING_BOLT]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    let plottable = |game: &Game| {
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::Plot { card } if *card == held))
    };
    assert!(plottable(&game), "your own main phase is the window");

    game.step = Step::Upkeep;
    assert!(!plottable(&game), "an upkeep is not");

    game.step = Step::PrecombatMain;
    game.active_player = PlayerId::Two;
    assert!(!plottable(&game), "and neither is their turn");
}

/// It is a special action rather than an ability, so nothing goes on the
/// stack for anyone to answer: the card is in exile the moment it is taken.
#[test]
fn plotting_puts_nothing_on_the_stack() {
    let (mut game, held, _) = staged(&[]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);

    let plot = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::Plot { card } if *card == held))
        .expect("two mana pays the plot cost");
    game.apply(PlayerId::One, plot).expect("it plots");

    assert!(game.stack.is_empty(), "no object was put on the stack");
    assert!(
        game.pending_triggers.is_empty(),
        "and nothing triggered off it",
    );
    assert_eq!(game.players[0].exile.len(), 1, "the card is simply gone");
}

/// "You may cast that card from exile ... during your main phase while the
/// stack is empty." A plotted card is a sorcery-speed cast on a later turn,
/// not a free instant.
#[test]
fn a_plotted_card_is_still_a_sorcery_speed_cast() {
    let (mut game, held, _) = staged(&[]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    let plot = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::Plot { card } if *card == held))
        .expect("two mana pays the plot cost");
    game.apply(PlayerId::One, plot).expect("it plots");
    let plotted = game.players[0].exile[0].id;

    game.commit_next_turn(PlayerId::Two, Vec::new());
    game.commit_next_turn(PlayerId::One, Vec::new());
    game.players[0].mana_pool = ManaPool::default();
    let castable = |game: &Game| {
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == plotted))
    };

    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    assert!(castable(&game), "your own main phase casts it");

    game.step = Step::DeclareBlockers;
    assert!(!castable(&game), "combat is not a window for it");

    game.step = Step::PrecombatMain;
    game.active_player = PlayerId::Two;
    assert!(
        !castable(&game),
        "and a free cast is not a free instant on their turn",
    );
}

/// "Whenever *you* cast a noncreature spell": their Bolt is not one of
/// yours, and the Bird is the 1/2 it was printed as.
#[test]
fn their_noncreature_spell_does_not_grow_it() {
    let (mut game, held, _others) = staged(&[]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    cast(&mut game, held);
    assert_eq!(game.power(bird(&game)), Some(1), "a 1/2 to begin with");

    let bolt = card(112_000, cards::LIGHTNING_BOLT, PlayerId::Two);
    let bolt_id = bolt.id;
    game.players[PlayerId::Two.index()].hand.push(bolt);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Red, 1);
    game.priority = PlayerId::Two;
    game.apply(
        PlayerId::Two,
        cast_action(bolt_id, vec![Target::Player(PlayerId::One)], Vec::new(), 0),
    )
    .expect("their Bolt is castable");
    settle(&mut game);

    assert_eq!(
        game.players[PlayerId::One.index()].life,
        17,
        "their spell resolved",
    );
    assert_eq!(
        game.power(bird(&game)),
        Some(1),
        "and grew nothing of yours doing it",
    );
}

/// The trigger is on casting rather than on resolving: a Bolt of yours that
/// is countered has still been cast, and the Bird keeps the two.
#[test]
fn a_countered_spell_of_yours_still_grows_it() {
    let (mut game, held, others) = staged(&[cards::LIGHTNING_BOLT]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 2);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    cast(&mut game, held);
    let counter = card(112_100, cards::COUNTERSPELL, PlayerId::Two);
    let counter_id = counter.id;
    game.players[PlayerId::Two.index()].hand.push(counter);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Blue, 2);
    game.priority = PlayerId::One;

    game.apply(
        PlayerId::One,
        cast_action(
            others[0],
            vec![Target::Player(PlayerId::Two)],
            Vec::new(),
            0,
        ),
    )
    .expect("your Bolt is castable");
    let bolt_spell = game
        .stack
        .iter()
        .next()
        .expect("the Bolt is on the stack")
        .id;
    game.priority = PlayerId::Two;
    let answer = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == counter_id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Spell(bolt_spell))
            }
            _ => false,
        })
        .expect("a Counterspell answers it");
    game.apply(PlayerId::Two, answer).expect("it is cast");
    settle(&mut game);

    assert_eq!(
        game.players[PlayerId::Two.index()].life,
        20,
        "the Bolt never resolved",
    );
    assert_eq!(
        game.power(bird(&game)),
        Some(3),
        "and casting it is all the trigger ever asked for",
    );
}
