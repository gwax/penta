//! Hexdrinker: a one-mana 2/1 that every spare mana turns further into
//! something the deck cannot answer.

use super::*;

/// Hexdrinker on the battlefield since last turn, with `mana` available and
/// a Bolt in the other player's hand.
fn staged(mana: u16) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let hexdrinker = game
        .put_onto_battlefield(PlayerId::One, cards::HEXDRINKER)
        .expect("cataloged");
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.players[1]
        .hand
        .push(card(96_500, cards::LIGHTNING_BOLT, PlayerId::Two));
    game.turns_started = [2, 1];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, mana);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Red, 1);
    (game, hexdrinker)
}

fn level_ups(game: &Game, hexdrinker: GameObjectId) -> Vec<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == hexdrinker),
        )
        .collect()
}

fn level_up(game: &mut Game, hexdrinker: GameObjectId) {
    let action = level_ups(game, hexdrinker)
        .into_iter()
        .next()
        .expect("leveling up is offered");
    game.apply(PlayerId::One, action).expect("it activates");
    for _ in 0..8 {
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

/// Sets the level directly, for the bands too far up to pay for in a test.
fn set_level(game: &mut Game, hexdrinker: GameObjectId, level: u16) {
    let permanent = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == hexdrinker)
        .expect("it is on the battlefield");
    permanent.add_counters(CounterKind::named("level"), level);
}

fn body(game: &Game, hexdrinker: GameObjectId) -> &Permanent {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == hexdrinker)
        .expect("it is on the battlefield")
}

/// Whether the other player's Bolt can be aimed at it.
fn bolt_can_hit(game: &Game, hexdrinker: GameObjectId) -> bool {
    let mut game = game.clone();
    game.priority = PlayerId::Two;
    game.legal_actions(PlayerId::Two).into_iter().any(|action| {
        matches!(
            action,
            Action::CastSpell { ref choices, .. }
                if choices
                    .iter_targets()
                    .any(|target| *target == Target::Permanent(hexdrinker))
        )
    })
}

/// Before level three it is what it prints, and every spare mana buys a
/// counter.
#[test]
fn it_starts_as_a_two_one_and_levels_one_counter_at_a_time() {
    let (mut game, hexdrinker) = staged(2);

    assert_eq!(game.power(body(&game, hexdrinker)), Some(2));
    assert_eq!(game.toughness(body(&game, hexdrinker)), Some(1));

    level_up(&mut game, hexdrinker);
    assert_eq!(
        body(&game, hexdrinker).counters(CounterKind::named("level")),
        1
    );
    assert_eq!(
        game.power(body(&game, hexdrinker)),
        Some(2),
        "one counter is not a band",
    );

    level_up(&mut game, hexdrinker);
    assert_eq!(
        body(&game, hexdrinker).counters(CounterKind::named("level")),
        2
    );
    assert!(
        level_ups(&game, hexdrinker).is_empty(),
        "and the mana is spent",
    );
}

/// "Level up only as a sorcery": their turn is closed, and so is a nonempty
/// stack.
#[test]
fn leveling_up_is_a_sorcery_speed_action() {
    let (mut game, hexdrinker) = staged(1);
    assert!(!level_ups(&game, hexdrinker).is_empty(), "your main phase");

    game.active_player = PlayerId::Two;
    assert!(
        level_ups(&game, hexdrinker).is_empty(),
        "their turn is closed",
    );

    game.active_player = PlayerId::One;
    game.step = Step::Upkeep;
    assert!(
        level_ups(&game, hexdrinker).is_empty(),
        "and so is your own upkeep",
    );
}

/// Three counters: a 4/4 that instants cannot touch.
#[test]
fn level_three_is_a_four_four_with_protection_from_instants() {
    let (mut game, hexdrinker) = staged(0);
    assert!(bolt_can_hit(&game, hexdrinker), "a 2/1 is Bolt food");

    set_level(&mut game, hexdrinker, 3);

    assert_eq!(game.power(body(&game, hexdrinker)), Some(4));
    assert_eq!(game.toughness(body(&game, hexdrinker)), Some(4));
    assert!(
        !bolt_can_hit(&game, hexdrinker),
        "protection from instants refuses the Bolt",
    );
    assert!(
        game.permanent_has_executable_keyword(
            body(&game, hexdrinker),
            KeywordAbility::ProtectionFrom(&crate::card::ObjectPredicateDef::HasType(
                crate::card::CardType::Instant
            )),
        ),
        "and it is the printed keyword rather than a lookalike",
    );
}

/// Seven is still the lower band; eight is the upper one.
#[test]
fn the_bands_do_not_overlap() {
    let (mut game, hexdrinker) = staged(0);
    set_level(&mut game, hexdrinker, 7);

    assert_eq!(game.power(body(&game, hexdrinker)), Some(4), "still 4/4");

    set_level(&mut game, hexdrinker, 1);

    assert_eq!(game.power(body(&game, hexdrinker)), Some(6), "now 6/6");
    assert_eq!(game.toughness(body(&game, hexdrinker)), Some(6));
}

/// Eight counters: nothing at all may target it.
#[test]
fn level_eight_is_protected_from_everything() {
    let (mut game, hexdrinker) = staged(0);
    set_level(&mut game, hexdrinker, 8);

    assert!(
        !bolt_can_hit(&game, hexdrinker),
        "no spell of any kind reaches it",
    );
    let bears = creature(96_600, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    let blocker = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == bears_id)
        .expect("the blocker is there");

    assert!(
        game.combat_is_protected(blocker, body(&game, hexdrinker)),
        "and nothing can block it either",
    );
}

/// Whether the other player's Flame Slash can be aimed at it. A sorcery
/// needs their turn and an empty stack, which is the only reason this is
/// not the same helper as the Bolt.
fn slash_can_hit(game: &Game, hexdrinker: GameObjectId) -> bool {
    let mut game = game.clone();
    game.players[1]
        .hand
        .push(card(96_700, cards::FLAME_SLASH, PlayerId::Two));
    game.active_player = PlayerId::Two;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::Two;
    game.legal_actions(PlayerId::Two).into_iter().any(|action| {
        matches!(
            action,
            Action::CastSpell { card, ref choices, .. }
                if card == CardInstanceId(96_700)
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(hexdrinker))
        )
    })
}

/// "Protection from instants means Hexdrinker can't be the target of instant
/// spells ... Nothing other than these events is prevented or illegal." A
/// sorcery is nothing other than these events, and four damage kills a 4/4.
#[test]
fn a_sorcery_still_answers_the_lower_band() {
    let (mut game, hexdrinker) = staged(0);
    set_level(&mut game, hexdrinker, 3);

    assert!(!bolt_can_hit(&game, hexdrinker), "an instant cannot");
    assert!(
        slash_can_hit(&game, hexdrinker),
        "and a sorcery is not what the protection names",
    );

    set_level(&mut game, hexdrinker, 5);

    assert!(
        !slash_can_hit(&game, hexdrinker),
        "at eight counters it is protected from everything, sorceries included",
    );
}

/// "If an effect has set Hexdrinker's power and/or toughness to a specific
/// value after it entered the battlefield, leveling up won't change that
/// characteristic." The band still hands over its keyword; what it cannot
/// do is win a layer it entered first.
#[test]
fn a_later_set_power_and_toughness_survives_leveling_up() {
    let (mut game, hexdrinker) = staged(0);
    attach_constant_resolved_characteristics(
        &mut game,
        hexdrinker,
        &[AppliedEffectDef::set_base_power_toughness(
            ValueDef::Constant(1),
            ValueDef::Constant(1),
        )],
        ContinuousEffectExpiration::Never,
    );
    assert_eq!(
        (
            game.power(body(&game, hexdrinker)),
            game.toughness(body(&game, hexdrinker))
        ),
        (Some(1), Some(1)),
        "the effect landed on a 2/1",
    );

    set_level(&mut game, hexdrinker, 3);

    assert_eq!(
        (
            game.power(body(&game, hexdrinker)),
            game.toughness(body(&game, hexdrinker))
        ),
        (Some(1), Some(1)),
        "the band would say 4/4 and it is later that says 1/1",
    );
    assert!(
        !bolt_can_hit(&game, hexdrinker),
        "and the half of the band that is not a size still arrives",
    );

    set_level(&mut game, hexdrinker, 5);

    assert_eq!(
        (
            game.power(body(&game, hexdrinker)),
            game.toughness(body(&game, hexdrinker))
        ),
        (Some(1), Some(1)),
        "eight counters do not move it either",
    );
}
