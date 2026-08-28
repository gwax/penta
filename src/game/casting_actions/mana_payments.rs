//! Locking announced branches of flexible mana symbols into one cast action.
//!
//! The mana planner may choose among mana alternatives, but paying life or
//! choosing the generic half of a two-brid symbol is an announced payment
//! choice. These helpers enumerate those choices and materialize the residual
//! mana cost before it reaches the planner.

use super::super::Game;
use crate::card::{FlexibleManaSymbol, ManaColor, ManaCost};
use crate::{FlexibleManaPayment, ManaPaymentChoice};

impl Game {
    /// Every explicitly announced flexible payment configuration for `cost`.
    pub(in crate::game) fn mana_payment_choices(cost: ManaCost) -> Vec<ManaPaymentChoice> {
        fn visit(
            cost: ManaCost,
            symbols: &[FlexibleManaSymbol],
            selected: &mut Vec<FlexibleManaPayment>,
            choices: &mut Vec<ManaPaymentChoice>,
        ) {
            let Some((symbol, rest)) = symbols.split_first() else {
                choices.push(ManaPaymentChoice::new(selected.clone()));
                return;
            };
            let count = cost.flexible_count(*symbol);
            if count == 0
                || (symbol.generic_alternative().is_none() && symbol.life_cost().is_none())
            {
                visit(cost, rest, selected, choices);
                return;
            }
            for alternative in 0..=count {
                if alternative > 0 {
                    selected.push(FlexibleManaPayment::new(*symbol, alternative));
                }
                visit(cost, rest, selected, choices);
                if alternative > 0 {
                    selected.pop();
                }
            }
        }

        let mut choices = Vec::new();
        visit(
            cost,
            &FlexibleManaSymbol::ALL,
            &mut Vec::new(),
            &mut choices,
        );
        choices
    }

    /// Resolves announced choices into a locked mana cost and life bill.
    ///
    /// Every copy of a two-brid symbol is locked: selected copies become two
    /// generic mana and the rest become the symbol's colored half. This keeps
    /// the mana solver from choosing the generic half again and collapsing two
    /// distinct legal actions into the same payment.
    /// `any_color` is "you may spend mana as though it were mana of any
    /// color to cast that spell": the coloured half of a flexible symbol
    /// stops being a colour and becomes an amount, exactly as the printed
    /// colours in the cost beside it already have. The symbol's other
    /// alternatives are untouched, so paying 2 life for a Phyrexian pip is
    /// still a line.
    pub(in crate::game) fn locked_mana_payment(
        mut cost: ManaCost,
        choice: &ManaPaymentChoice,
        any_color: bool,
    ) -> Option<(ManaCost, u16)> {
        let mut seen = Vec::new();
        for payment in choice.alternatives() {
            if payment.count() == 0 || seen.contains(&payment.symbol()) {
                return None;
            }
            let symbol = payment.symbol();
            if symbol.generic_alternative().is_none() && symbol.life_cost().is_none() {
                return None;
            }
            seen.push(symbol);
        }

        let mut life = 0_u16;
        for symbol in FlexibleManaSymbol::ALL {
            let printed = cost.flexible_count(symbol);
            let selected = choice
                .alternatives()
                .iter()
                .find(|payment| payment.symbol() == symbol)
                .map_or(0, |payment| payment.count());
            if selected > printed {
                return None;
            }
            if let Some(generic) = symbol.generic_alternative() {
                cost = cost.without_flexible(symbol, printed)?;
                cost.generic = cost.generic.checked_add(generic.checked_mul(selected)?)?;
                let colored = printed - selected;
                if any_color {
                    cost.generic = cost.generic.checked_add(colored)?;
                } else {
                    let color = *symbol.mana_options().first()?;
                    add_fixed_mana(&mut cost, color, colored)?;
                }
            } else if let Some(amount) = symbol.life_cost() {
                cost = cost.without_flexible(symbol, selected)?;
                life = life.checked_add(amount.checked_mul(selected)?)?;
                if any_color {
                    let colored = printed - selected;
                    cost = cost.without_flexible(symbol, colored)?;
                    cost.generic = cost.generic.checked_add(colored)?;
                }
            } else if selected != 0 {
                return None;
            }
        }
        Some((cost, life))
    }

    pub(in crate::game) fn mana_payment_life(choice: &ManaPaymentChoice) -> Option<u16> {
        choice
            .alternatives()
            .iter()
            .try_fold(0_u16, |total, payment| {
                if let Some(amount) = payment.symbol().life_cost() {
                    total.checked_add(amount.checked_mul(payment.count())?)
                } else if payment.symbol().generic_alternative().is_some() {
                    Some(total)
                } else {
                    None
                }
            })
    }

    /// Number of Phyrexian symbols whose announced payment was 2 life.
    /// Compleated reduces starting loyalty once for each such symbol, not
    /// merely once for the whole spell.
    pub(in crate::game) fn phyrexian_symbols_paid_with_life(
        choice: &ManaPaymentChoice,
    ) -> Option<u16> {
        choice
            .alternatives()
            .iter()
            .try_fold(0_u16, |total, payment| {
                if payment.symbol().life_cost().is_some() {
                    total.checked_add(payment.count())
                } else if payment.symbol().generic_alternative().is_some() {
                    Some(total)
                } else {
                    None
                }
            })
    }
}

fn add_fixed_mana(cost: &mut ManaCost, color: ManaColor, amount: u16) -> Option<()> {
    let slot = match color {
        ManaColor::White => &mut cost.white,
        ManaColor::Blue => &mut cost.blue,
        ManaColor::Black => &mut cost.black,
        ManaColor::Red => &mut cost.red,
        ManaColor::Green => &mut cost.green,
        ManaColor::Colorless => &mut cost.colorless,
    };
    *slot = slot.checked_add(amount)?;
    Some(())
}
