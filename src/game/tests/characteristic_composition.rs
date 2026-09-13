use super::*;
use crate::card::CharacteristicPredicateDef;

const HAS_ADVENTURE: ObjectPredicateDef = ObjectPredicateDef::HasAlternativeCharacteristics(
    CharacteristicPredicateDef::Subtype("Adventure"),
);

#[test]
fn alternative_sets_follow_copiable_presentation_and_preserve_snapshots() {
    let mut game = ready_game();
    let original = creature(91_060, cards::BRAZEN_BORROWER, PlayerId::One);
    for context in [
        CharacteristicContext::Hand,
        CharacteristicContext::Library,
        CharacteristicContext::Graveyard,
        CharacteristicContext::Exile,
        CharacteristicContext::Stack {
            form: SpellForm::Part(CardPartId::PRIMARY),
        },
    ] {
        let view = game
            .printed_trigger_event_object(
                original.card.id,
                cards::BRAZEN_BORROWER,
                PlayerId::One,
                &context,
            )
            .unwrap();
        assert!(game.trigger_object_matches_for_controller(
            HAS_ADVENTURE,
            &view,
            original.card.id,
            false,
            Some(PlayerId::One)
        ));
        assert!(!view.subtypes.contains(&"Adventure"));
    }
    let mut copier = creature(91_061, cards::GRIZZLY_BEARS, PlayerId::One);
    copier.copy_effect = Some(copied_characteristics(cards::BRAZEN_BORROWER));
    let mut token_copy = token_permanent(91_062, tokens::food(), PlayerId::One);
    token_copy.copy_effect = Some(copied_characteristics(cards::BRAZEN_BORROWER));
    game.battlefield.extend([original, copier, token_copy]);
    assert!(
        !game
            .trigger_event_object(&game.battlefield[2])
            .alternative_characteristics
            .is_empty()
    );
    let snapshot = game.trigger_event_object(&game.battlefield[1]);
    assert!(!snapshot.alternative_characteristics.is_empty());
    game.battlefield[1].copy_effect = None;
    assert!(
        game.trigger_event_object(&game.battlefield[1])
            .alternative_characteristics
            .is_empty()
    );
    assert!(game.trigger_object_matches_for_controller(
        HAS_ADVENTURE,
        &snapshot,
        snapshot.id,
        false,
        Some(PlayerId::One)
    ));
    game.battlefield[0].face_down = Some(crate::card::face_down::ordinary());
    assert!(
        game.trigger_event_object(&game.battlefield[0])
            .alternative_characteristics
            .is_empty()
    );
    assert!(
        Game::face_down_exiled_event_object(snapshot.id, PlayerId::One)
            .alternative_characteristics
            .is_empty()
    );
}

#[test]
fn an_alternative_spell_uses_its_own_types_and_has_no_parent_alternative_set() {
    let game = ready_game();
    let view = game
        .printed_trigger_event_object(
            GameObjectId(91_080),
            cards::BRAZEN_BORROWER,
            PlayerId::One,
            &CharacteristicContext::Stack {
                form: SpellForm::Part(CardPartId(1)),
            },
        )
        .unwrap();
    assert!(view.types.contains(CardType::Instant));
    assert!(view.subtypes.contains(&"Adventure"));
    assert!(!game.trigger_object_matches_for_controller(
        HAS_ADVENTURE,
        &view,
        view.id,
        true,
        Some(PlayerId::One)
    ));
}

#[test]
fn alternative_queries_compose_types_colors_and_names_without_granting_casts() {
    let game = ready_game();
    let normal = game
        .printed_trigger_event_object(
            GameObjectId(91_081),
            cards::BRAZEN_BORROWER,
            PlayerId::One,
            &CharacteristicContext::Graveyard,
        )
        .unwrap();
    assert!(game.alternative_characteristics_match(
        &normal.alternative_characteristics,
        CharacteristicPredicateDef::All(&[
            CharacteristicPredicateDef::Name("Petty Theft"),
            CharacteristicPredicateDef::HasType(CardType::Instant),
            CharacteristicPredicateDef::Color(ManaColor::Blue),
            CharacteristicPredicateDef::ManaValueAtMost(2),
            CharacteristicPredicateDef::Not(&CharacteristicPredicateDef::HasType(
                CardType::Creature
            )),
        ])
    ));
    assert!(!game.alternative_characteristics_match(
        &normal.alternative_characteristics,
        CharacteristicPredicateDef::Subtype("Omen")
    ));
}

#[test]
fn contributed_spell_text_outlives_its_donor_object() {
    let mut game = ready_game();
    let donor = card(91_090, cards::THROUGH_THE_BREACH, PlayerId::One);
    let donor_id = donor.id;
    game.players[0].hand.push(donor);
    let announced = game
        .spliced_spell_clauses(PlayerId::One, &[donor_id])
        .unwrap();
    game.discard_cards(PlayerId::One, &[donor_id]);
    assert!(game.card_in_nonbattlefield_zone(donor_id).is_none());
    assert_eq!(game.spliced_clauses_of(&[donor_id]).unwrap(), announced);
    assert!(
        game.spliced_spell_clauses(PlayerId::One, &[donor_id])
            .is_none()
    );
}
