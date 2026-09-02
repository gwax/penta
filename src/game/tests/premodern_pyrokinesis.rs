//! Pyrokinesis: a fixed total split among as many creatures as you like.
//!
//! The card prints no ceiling on the number of targets, but the division is
//! its own ceiling -- every target must be assigned at least one damage.

use super::*;

fn ready() -> Game {
    let mut game = ready_game();
    game.turn = 5;
    game.turns_started[PlayerId::One.index()] = 5;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game
}

fn resolve(game: &mut Game) {
    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
}

/// Pyrokinesis splits a fixed four damage among however many creatures the
/// caster names. There is no printed ceiling, but the division supplies one:
/// every target takes at least one, so four targets is the most it reaches.
#[test]
fn pyrokinesis_divides_four_damage_and_cannot_name_a_fifth_creature() {
    let mut game = ready();
    for index in 0..5 {
        game.battlefield
            .push(creature(10_000 + index, cards::SERRA_ANGEL, PlayerId::Two));
    }
    let pyro = card(20_000, cards::PYROKINESIS, PlayerId::One);
    let pyro_id = pyro.id;
    game.players[PlayerId::One.index()].hand.push(pyro);
    game.players[PlayerId::One.index()].mana_pool.red = 2;
    game.players[PlayerId::One.index()].mana_pool.colorless = 4;

    let widest = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell { card, choices, .. } if card == pyro_id => Some(
                choices
                    .targets()
                    .iter()
                    .map(|slot| slot.targets().len())
                    .sum::<usize>(),
            ),
            _ => None,
        })
        .max()
        .expect("Pyrokinesis is castable");
    assert_eq!(widest, 4, "four damage cannot be split more than four ways");
}

/// And the four damage actually lands, split across the creatures named.
#[test]
fn pyrokinesis_deals_its_four_damage() {
    let mut game = ready();
    game.battlefield
        .push(creature(10_000, cards::SERRA_ANGEL, PlayerId::Two));
    let pyro = card(20_000, cards::PYROKINESIS, PlayerId::One);
    let pyro_id = pyro.id;
    game.players[PlayerId::One.index()].hand.push(pyro);
    game.players[PlayerId::One.index()].mana_pool.red = 2;
    game.players[PlayerId::One.index()].mana_pool.colorless = 4;

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == pyro_id))
        .expect("Pyrokinesis is castable");
    game.apply(PlayerId::One, cast).expect("it is cast");
    resolve(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == CardInstanceId(10_000)),
        "all four went to the lone target, which kills a 4/4",
    );
}

/// "You may exile a red card from your hand rather than pay this spell's
/// mana cost." The reason the card sees play at all, and neither test above
/// touches it: with a red card beside it and no mana anywhere, it is still
/// castable, and the red card leaves the hand for exile.
#[test]
fn a_red_card_out_of_hand_casts_it_for_nothing() {
    let mut game = ready();
    game.battlefield
        .push(creature(10_000, cards::SERRA_ANGEL, PlayerId::Two));
    let pyro = card(20_000, cards::PYROKINESIS, PlayerId::One);
    let pyro_id = pyro.id;
    game.players[PlayerId::One.index()].hand.push(pyro);

    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == pyro_id)),
        "no mana and nothing red to pitch buys nothing",
    );

    game.players[PlayerId::One.index()].hand.push(card(
        20_001,
        cards::LIGHTNING_BOLT,
        PlayerId::One,
    ));

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == pyro_id))
        .expect("a red card in hand is the whole cost");
    game.apply(PlayerId::One, cast).expect("it is cast");
    resolve(&mut game);

    assert_eq!(
        game.players[PlayerId::One.index()].mana_pool.total(),
        0,
        "and it cost no mana, because there was none to cost",
    );
    assert!(
        game.players[PlayerId::One.index()]
            .exile
            .iter()
            .any(|card| card.definition == cards::LIGHTNING_BOLT),
        "the Bolt was exiled rather than discarded",
    );
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == CardInstanceId(10_000)),
        "and the Angel took all four",
    );
}

/// "If another effect causes Pyrokinesis to cost more, you must pay that
/// additional cost even if you pay its alternative cost." A Thalia taxes
/// noncreature spells, and free is not exempt: the pitch still happens and
/// one mana has to come from somewhere on top of it.
#[test]
fn a_tax_is_paid_on_top_of_the_free_cast() {
    let mut game = ready();
    game.battlefield
        .push(creature(10_000, cards::SERRA_ANGEL, PlayerId::Two));
    game.battlefield.push(creature(
        10_001,
        cards::THALIA_GUARDIAN_OF_THRABEN,
        PlayerId::Two,
    ));
    let pyro = card(20_000, cards::PYROKINESIS, PlayerId::One);
    let pyro_id = pyro.id;
    game.players[PlayerId::One.index()].hand.push(pyro);
    game.players[PlayerId::One.index()].hand.push(card(
        20_001,
        cards::LIGHTNING_BOLT,
        PlayerId::One,
    ));

    let castable = |game: &Game| {
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == pyro_id))
    };
    assert!(
        !castable(&game),
        "a red card alone no longer covers it while she is out",
    );

    game.players[PlayerId::One.index()].mana_pool.colorless = 1;

    assert!(
        castable(&game),
        "the pitch plus her one mana is what it costs now",
    );
}

/// "You divide the damage as you cast Pyrokinesis, not as it resolves. If
/// any of the targets become illegal, damage is dealt to the other targets
/// as originally assigned." Two Bears named for two apiece: kill one in
/// response and the other still takes the two it was assigned, not the four
/// that is now going nowhere else.
#[test]
fn a_target_that_leaves_takes_only_its_own_share_with_it() {
    let mut game = ready();
    let doomed = creature(10_100, cards::SERRA_ANGEL, PlayerId::Two);
    let doomed_id = doomed.card.id;
    game.battlefield.push(doomed);
    let survivor = creature(10_101, cards::SERRA_ANGEL, PlayerId::Two);
    let survivor_id = survivor.card.id;
    game.battlefield.push(survivor);
    let pyro = card(20_100, cards::PYROKINESIS, PlayerId::One);
    let pyro_id = pyro.id;
    game.players[PlayerId::One.index()].hand.push(pyro);
    game.players[PlayerId::One.index()].mana_pool.red = 2;
    game.players[PlayerId::One.index()].mana_pool.colorless = 4;

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == pyro_id
                    && choices.targets().iter().any(|slot| {
                        slot.amount_for(Target::Permanent(doomed_id)) == Some(2)
                            && slot.amount_for(Target::Permanent(survivor_id)) == Some(2)
                    })
            }
            _ => false,
        })
        .expect("two apiece across the two Angels");
    game.apply(PlayerId::One, cast).expect("it is cast");

    // One of them is answered while the spell waits.
    game.move_permanents_to_graveyard(&[doomed_id]);
    game.check_state_based_actions();
    resolve(&mut game);
    game.check_state_based_actions();

    let survivor = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == survivor_id)
        .expect("a 4/4 that took two is still standing");
    assert_eq!(
        survivor.damage, 2,
        "its own share and no more: the other two went with the Angel that left",
    );
}
