use crate::{CardPartId, MeldRecipeId};

use super::DoubleFacedKind;

/// A predicate over a characteristic set, independent of an object's zone,
/// controller, combat state, physical backing, or casting permissions.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CharacteristicPredicateDef {
    Any,
    Name(&'static str),
    HasType(super::CardType),
    Subtype(&'static str),
    Supertype(super::CardSupertype),
    Color(super::ManaColor),
    ManaValueAtMost(u16),
    HasKeyword(super::KeywordAbility),
    All(&'static [Self]),
    AnyOf(&'static [Self]),
    Not(&'static Self),
}

impl CharacteristicPredicateDef {
    #[must_use]
    pub fn matches(self, part: &super::CardPart) -> bool {
        match self {
            Self::Any => true,
            Self::Name(name) => part.name == name,
            Self::HasType(kind) => part.rules.has_type(kind),
            Self::Subtype(subtype) => part.rules.has_subtype(subtype),
            Self::Supertype(supertype) => part.rules.has_supertype(supertype),
            Self::Color(color) => color
                .color_index()
                .is_some_and(|index| part.rules.colors()[index]),
            Self::ManaValueAtMost(value) => part.rules.printed_mana_cost().mana_value() <= value,
            Self::HasKeyword(keyword) => part.rules.has_keyword(keyword),
            Self::All(predicates) => predicates.iter().all(|predicate| predicate.matches(part)),
            Self::AnyOf(predicates) => predicates.iter().any(|predicate| predicate.matches(part)),
            Self::Not(predicate) => !predicate.matches(part),
        }
    }
}

/// Physical faces are independent of the characteristics an object presents
/// and the play options its rules allow. In particular, a modal double-faced
/// permanent may transform (CR 712.3), and an inset is not a physical back face.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CardFaces {
    Single,
    Double {
        front: CardPartId,
        back: CardPartId,
        kind: DoubleFacedKind,
    },
    MeldComponent {
        front: CardPartId,
        recipe: MeldRecipeId,
    },
}

/// A characteristic expression is also useful outside casting: split cards
/// combine their halves in a graveyard, even when no combined cast is allowed.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum CharacteristicExpression {
    Part(CardPartId),
    Combined(Vec<CardPartId>),
}

impl CharacteristicExpression {
    #[must_use]
    pub fn parts(&self) -> &[CardPartId] {
        match self {
            Self::Part(part) => std::slice::from_ref(part),
            Self::Combined(parts) => parts,
        }
    }
}

/// The fields supplied by an alternative set; other fields are inherited
/// from its normal set. This represents partial alternatives such as flip
/// cards and prototype without duplicating their unchanged characteristics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CharacteristicField {
    Name,
    ManaCost,
    Color,
    TypeLine,
    RulesText,
    PowerToughness,
    Loyalty,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AlternativeCharacteristics {
    pub normal: CardPartId,
    pub alternative: CardPartId,
    /// `None` supplies the complete set. A list replaces exactly those fields.
    pub fields: Option<Vec<CharacteristicField>>,
}

/// State transitions select battlefield presentations. Casting permissions
/// remain in play options and abilities, rather than in this state model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BattlefieldPresentation {
    Fixed(CardPartId),
    Flip {
        normal: CardPartId,
        flipped: CardPartId,
    },
    Unlock {
        doors: Vec<CardPartId>,
        combined: CardPartId,
        locked: CardPartId,
    },
}

impl BattlefieldPresentation {
    #[must_use]
    pub const fn initial(&self) -> CardPartId {
        match self {
            Self::Fixed(part) => *part,
            Self::Flip { normal, .. } => *normal,
            Self::Unlock { locked, .. } => *locked,
        }
    }
}

/// Relationships among characteristic sets, physical faces, and battlefield
/// presentations. None of these relationships by itself grants a play action.
/// An Adventure, an Omen, and a preparation inset use the same alternative
/// relationship; their play and resolution programs determine its use.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardStructure {
    pub parts: Vec<CardPartId>,
    pub normal: CharacteristicExpression,
    pub alternatives: Vec<AlternativeCharacteristics>,
    pub faces: CardFaces,
    pub battlefield: BattlefieldPresentation,
}

impl CardStructure {
    #[must_use]
    pub fn single(main: CardPartId) -> Self {
        Self {
            parts: vec![main],
            normal: CharacteristicExpression::Part(main),
            alternatives: Vec::new(),
            faces: CardFaces::Single,
            battlefield: BattlefieldPresentation::Fixed(main),
        }
    }

    #[must_use]
    pub fn with_alternative(self, normal: CardPartId, alternative: CardPartId) -> Self {
        self.with_alternative_fields(normal, alternative, None)
    }

    #[must_use]
    pub fn with_alternative_fields(
        mut self,
        normal: CardPartId,
        alternative: CardPartId,
        fields: Option<Vec<CharacteristicField>>,
    ) -> Self {
        self.parts.push(alternative);
        self.alternatives.push(AlternativeCharacteristics {
            normal,
            alternative,
            fields,
        });
        self
    }

    #[must_use]
    pub fn split(parts: Vec<CardPartId>) -> Self {
        let mut structure = Self::single(parts.first().copied().unwrap_or(CardPartId::PRIMARY));
        structure.normal = CharacteristicExpression::Combined(parts.clone());
        structure.parts = parts;
        structure
    }

    #[must_use]
    pub fn double_faced(front: CardPartId, back: CardPartId, kind: DoubleFacedKind) -> Self {
        let mut structure = Self::single(front);
        structure.parts.push(back);
        structure.faces = CardFaces::Double { front, back, kind };
        structure
    }

    #[must_use]
    pub fn flip(normal: CardPartId, flipped: CardPartId) -> Self {
        let mut structure = Self::single(normal).with_alternative_fields(
            normal,
            flipped,
            Some(vec![
                CharacteristicField::Name,
                CharacteristicField::TypeLine,
                CharacteristicField::RulesText,
                CharacteristicField::PowerToughness,
            ]),
        );
        structure.battlefield = BattlefieldPresentation::Flip { normal, flipped };
        structure
    }

    #[must_use]
    pub fn room(doors: Vec<CardPartId>, combined: CardPartId, locked: CardPartId) -> Self {
        let mut structure = Self::split(doors.clone());
        structure.parts.extend([combined, locked]);
        structure.battlefield = BattlefieldPresentation::Unlock {
            doors,
            combined,
            locked,
        };
        structure
    }

    #[must_use]
    pub fn meld_component(front: CardPartId, recipe: MeldRecipeId) -> Self {
        let mut structure = Self::single(front);
        structure.faces = CardFaces::MeldComponent { front, recipe };
        structure
    }

    pub fn alternatives_for(&self, normal: CardPartId) -> impl Iterator<Item = CardPartId> + '_ {
        self.alternatives
            .iter()
            .filter_map(move |set| (set.normal == normal).then_some(set.alternative))
    }
}
