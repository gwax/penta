use std::ops::{Index, Range};

use super::AbilityDef;

/// An ordered, borrowed view of a rules program. Joining printed components
/// preserves their clauses without allocating or copying a second ability tree.
#[derive(Clone, Copy, Debug)]
pub enum AbilityClauses<'a> {
    Slice(&'a [AbilityDef]),
    Joined(&'static super::CardRules, &'static super::CardRules),
}

impl<'a> AbilityClauses<'a> {
    #[must_use]
    pub fn len(self) -> usize {
        match self {
            Self::Slice(abilities) => abilities.len(),
            Self::Joined(left, right) => {
                left.ability_clauses().len() + right.ability_clauses().len()
            }
        }
    }

    #[must_use]
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    #[must_use]
    pub fn get(self, index: usize) -> Option<&'a AbilityDef> {
        match self {
            Self::Slice(abilities) => abilities.get(index),
            Self::Joined(left, right) => {
                let left = left.ability_clauses();
                if index < left.len() {
                    left.get(index)
                } else {
                    right.ability_clauses().get(index - left.len())
                }
            }
        }
    }

    #[must_use]
    pub fn first(self) -> Option<&'a AbilityDef> {
        self.get(0)
    }

    #[must_use]
    pub fn iter(self) -> AbilityClauseIter<'a> {
        AbilityClauseIter {
            clauses: self,
            indices: 0..self.len(),
        }
    }

    #[must_use]
    pub fn to_vec(self) -> Vec<AbilityDef> {
        self.iter().copied().collect()
    }
}

impl Index<usize> for AbilityClauses<'_> {
    type Output = AbilityDef;
    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).expect("ability clause index is in bounds")
    }
}

impl<'a> IntoIterator for AbilityClauses<'a> {
    type Item = &'a AbilityDef;
    type IntoIter = AbilityClauseIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct AbilityClauseIter<'a> {
    clauses: AbilityClauses<'a>,
    indices: Range<usize>,
}

impl<'a> Iterator for AbilityClauseIter<'a> {
    type Item = &'a AbilityDef;
    fn next(&mut self) -> Option<Self::Item> {
        self.clauses.get(self.indices.next()?)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.indices.size_hint()
    }
}

impl DoubleEndedIterator for AbilityClauseIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.clauses.get(self.indices.next_back()?)
    }
}

impl ExactSizeIterator for AbilityClauseIter<'_> {}

impl<'a> From<&'a [AbilityDef]> for AbilityClauses<'a> {
    fn from(abilities: &'a [AbilityDef]) -> Self {
        Self::Slice(abilities)
    }
}
