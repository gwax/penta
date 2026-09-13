//! Face-up exile targeting and owner-directed graveyard placement.

use super::*;

fn pull_game(prepared: bool) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.set_prepared_engine_enabled(prepared);
    let pull = card(20_000, cards::PULL_FROM_ETERNITY, PlayerId::One);
    let id = pull.id;
    game.players[0].hand.push(pull);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    (game, id)
}

#[test]
fn pull_from_eternity_moves_either_owners_face_up_card_to_their_graveyard() {
    for prepared in [false, true] {
        for owner in [PlayerId::One, PlayerId::Two] {
            let (mut game, pull) = pull_game(prepared);
            // Lands qualify too: the target has no card-type restriction.
            let exiled = card(20_001, cards::FOREST, owner);
            let target = exiled.id;
            game.players[owner.index()].exile.push(exiled);
            let cast = cast_action(pull, vec![Target::Card(target)], Vec::new(), 0);
            assert!(game.legal_actions(PlayerId::One).contains(&cast));
            game.apply(PlayerId::One, cast).unwrap();
            pass_priority_pair(&mut game);

            assert!(game.players[owner.index()].exile.is_empty());
            let moved = game.players[owner.index()]
                .graveyard
                .iter()
                .find(|card| card.definition == cards::FOREST)
                .expect("the card goes to its owner's graveyard");
            assert_ne!(moved.id, target);
            assert_eq!(moved.owner, owner);
            assert!(
                game.players[0]
                    .graveyard
                    .iter()
                    .any(|card| card.definition == cards::PULL_FROM_ETERNITY)
            );
        }
    }
}

#[test]
fn pull_from_eternity_rejects_face_down_exiles_even_with_permission_to_look() {
    let (mut game, pull) = pull_game(true);
    for (index, owner) in [PlayerId::One, PlayerId::Two].into_iter().enumerate() {
        for may_look in [false, true] {
            let exiled = card(
                20_010 + u32::try_from(index).unwrap() * 2 + u32::from(may_look),
                cards::FOREST,
                owner,
            );
            let target = exiled.id;
            game.players[owner.index()].exile.push(exiled);
            if may_look {
                game.permit_look_while_exiled(target, PlayerId::One);
            } else {
                game.hide_from_everyone_while_exiled(target, owner);
            }
            let cast = cast_action(pull, vec![Target::Card(target)], Vec::new(), 0);
            assert!(!game.legal_actions(PlayerId::One).contains(&cast));
            assert!(game.apply(PlayerId::One, cast).is_err());
        }
    }
    let ordinary = card(20_020, cards::FOREST, PlayerId::One);
    let target = ordinary.id;
    game.players[0].graveyard.push(ordinary);
    assert!(!game.legal_actions(PlayerId::One).contains(&cast_action(
        pull,
        vec![Target::Card(target)],
        Vec::new(),
        0,
    )));
}

#[test]
fn pull_from_eternity_rechecks_exile_facing_and_zone_identity_on_resolution() {
    for prepared in [false, true] {
        for turn_face_down in [false, true] {
            let (mut game, pull) = pull_game(prepared);
            let exiled = card(20_030, cards::FOREST, PlayerId::Two);
            let target = exiled.id;
            game.players[1].exile.push(exiled);
            game.apply(
                PlayerId::One,
                cast_action(pull, vec![Target::Card(target)], Vec::new(), 0),
            )
            .unwrap();
            if turn_face_down {
                game.permit_look_while_exiled(target, PlayerId::One);
            } else {
                let (moved, _) = game
                    .move_card_from_nonbattlefield_zone(
                        target,
                        ZoneKind::Exile,
                        ZoneKind::Hand,
                        ZoneMoveCause::Rules,
                        None,
                    )
                    .unwrap();
                game.move_card_from_nonbattlefield_zone(
                    moved.id,
                    ZoneKind::Hand,
                    ZoneKind::Exile,
                    ZoneMoveCause::Rules,
                    None,
                )
                .unwrap();
            }
            pass_priority_pair(&mut game);
            assert_eq!(game.players[1].exile.len(), 1);
            assert!(game.players[1].graveyard.is_empty());
            assert!(game.stack.is_empty());
        }
    }
}

#[test]
fn pull_from_eternity_can_target_a_face_down_permanent_after_normal_exile() {
    for prepared in [false, true] {
        let (mut game, pull) = pull_game(prepared);
        let mut permanent = creature(20_040, cards::KROSAN_COLOSSUS, PlayerId::Two);
        permanent.face_down = Some(crate::card::face_down::morph());
        let id = permanent.card.id;
        game.battlefield.push(permanent);
        game.exile_permanent(id);

        let exiled = game.players[1].exile[0].id;
        assert!(!game.exiled_card_is_face_down(exiled));
        let cast = cast_action(pull, vec![Target::Card(exiled)], Vec::new(), 0);
        assert!(game.legal_actions(PlayerId::One).contains(&cast));
        game.apply(PlayerId::One, cast).unwrap();
        pass_priority_pair(&mut game);
        assert!(game.players[1].exile.is_empty());
        assert!(
            game.players[1]
                .graveyard
                .iter()
                .any(|card| card.definition == cards::KROSAN_COLOSSUS)
        );
    }
}

#[test]
fn face_up_in_exile_requires_live_exile_facing_instead_of_permanent_status() {
    let mut game = ready_game();
    let card = card(20_050, cards::FOREST, PlayerId::One);
    for zone in [
        ZoneKind::Library,
        ZoneKind::Hand,
        ZoneKind::Graveyard,
        ZoneKind::Command,
    ] {
        assert!(!game.card_object_matches(ObjectPredicateDef::FaceUpInExile, &card, zone, card.id));
    }
    for face_down in [false, true] {
        let mut permanent = creature(20_051, cards::KROSAN_COLOSSUS, PlayerId::One);
        permanent.face_down = face_down.then(crate::card::face_down::morph);
        game.battlefield.push(permanent);
        let snapshot = game.targeting_event_object(game.battlefield.last().unwrap());
        assert!(!game.trigger_object_matches(
            ObjectPredicateDef::FaceUpInExile,
            &snapshot,
            card.id,
            false
        ));
        game.battlefield.clear();

        let mut spell = spell(20_052, cards::KROSAN_COLOSSUS, PlayerId::One, 0);
        spell.face_down = face_down.then(crate::card::face_down::morph);
        let snapshot = game.stack_object_event_object(&spell).unwrap();
        assert!(!game.trigger_object_matches(
            ObjectPredicateDef::FaceUpInExile,
            &snapshot,
            card.id,
            true
        ));
    }
}
