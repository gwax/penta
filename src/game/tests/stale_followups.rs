//! Identities unblocked by earlier passes rather than by new machinery.
//!
//! Each of these pairs something built earlier in this session with something
//! that already existed. The tests drive the pairing rather than either half:
//! a counter-conditional grant on an unleash creature, and a sacrifice
//! ability reading the power of the creature it just spent.

use super::*;
use crate::ImplementationStatus;

/// Chaos Imps has trample only while it carries the unleash counter, so the
/// two halves of the card interact rather than sitting side by side.
#[test]
fn chaos_imps_gains_trample_only_with_its_counter() {
    let mut game = ready_game();
    let imps = creature(10_000, cards::CHAOS_IMPS, PlayerId::One);
    let imps_id = imps.card.id;
    game.battlefield.push(imps);

    let without = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == imps_id)
        .expect("there");
    assert!(
        !game.permanent_has_executable_keyword(without, KeywordAbility::Trample),
        "no counter, no trample"
    );

    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == imps_id)
        .expect("there")
        .add_counters(CounterKind::PlusOnePlusOne, 1);

    let with = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == imps_id)
        .expect("there");
    assert!(
        game.permanent_has_executable_keyword(with, KeywordAbility::Trample),
        "the unleash counter is what turns it on"
    );
}

/// Hellhole Flailer sacrifices itself and then reads its own power, which
/// only works from last known information.
#[test]
fn hellhole_flailer_deals_its_power_after_sacrificing_itself() {
    let mut game = ready_game();
    let flailer = creature(10_000, cards::HELLHOLE_FLAILER, PlayerId::One);
    let flailer_id = flailer.card.id;
    game.battlefield.push(flailer);
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == flailer_id)
        .expect("there")
        // Unleashed, so it is a 4/3 rather than the printed 3/2.
        .add_counters(CounterKind::PlusOnePlusOne, 1);
    let pool = &mut game.players[PlayerId::One.index()].mana_pool;
    pool.black = 1;
    pool.red = 1;
    pool.colorless = 2;
    let before = game.players[PlayerId::Two.index()].life;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } => {
                *source == flailer_id
                    && targets
                        .iter()
                        .flat_map(crate::casting::TargetSelection::targets)
                        .any(|target| *target == Target::Player(PlayerId::Two))
            }
            _ => false,
        })
        .expect("it can be sacrificed at the other player");
    game.apply(PlayerId::One, action)
        .expect("the ability activates");
    drain_pending(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == flailer_id),
        "it spent itself"
    );
    assert_eq!(
        game.players[PlayerId::Two.index()].life,
        before - 4,
        "and dealt the four power it had when it left"
    );
}

#[test]
fn every_unblocked_identity_reports_complete_coverage() {
    let catalog = poc::catalog().expect("catalog builds");
    for definition in [
        cards::GRIM_ROUSTABOUT,
        cards::CHAOS_IMPS,
        cards::HELLHOLE_FLAILER,
        cards::ACCORDERS_SHIELD,
        cards::FIRESHRIEKER,
    ] {
        let card = catalog.get(definition).expect("the card is cataloged");
        assert_eq!(
            card.rules.implementation_status(),
            ImplementationStatus::Complete,
            "{} should be fully executable",
            card.name,
        );
    }
}
