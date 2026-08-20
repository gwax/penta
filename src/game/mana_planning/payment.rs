// Spending a pool against one cost.
//
// Separate from the planning above because it answers a different question:
// planning decides which sources to tap and whether a cast is affordable at
// all, while this decides which mana in the pool actually leaves it.
// Included textually into `mana_planning.rs`, so the imports here are the
// parent module's.

#[cfg(test)]
pub(super) fn pay_cost(pool: &mut ManaPool, cost: ManaCost, x: u16) {
    pay_cost_with_generic_strategy(
        pool,
        cost,
        x,
        // No rider to prefer, so each pair spends in its printed order.
        &|_| false,
        &[
            ManaColor::Colorless,
            ManaColor::Green,
            ManaColor::Black,
            ManaColor::Red,
            ManaColor::White,
            ManaColor::Blue,
        ],
        false,
    );
}

/// Spends a pool against one cost. `spread_generic_colors` pays the generic
/// portion across as many colours as it can instead of draining them in
/// order, which is what converge wants and nothing else does.
pub(super) fn pay_cost_with_generic_strategy(
    pool: &mut ManaPool,
    cost: ManaCost,
    x: u16,
    hybrid_preference: &impl Fn(ManaColor) -> bool,
    generic_order: &[ManaColor],
    spread_generic_colors: bool,
) {
    for color in colored_mana() {
        pool.remove_color(color, mana_cost_amount(cost, color));
    }
    for pair in HybridPair::ALL {
        let mut remaining = cost.hybrid[pair.index()];
        if remaining == 0 {
            continue;
        }
        let (first, second) = pair.colors();
        let mut order = [first, second];
        order.sort_by_key(|color| hybrid_preference(*color));
        for color in order {
            let spent = pool.amount(color).min(remaining);
            pool.remove_color(color, spent);
            remaining -= spent;
            if remaining == 0 {
                break;
            }
        }
        debug_assert_eq!(remaining, 0);
    }
    let generic = cost
        .generic
        .saturating_add(x.saturating_mul(cost.x_multiplier));
    if spread_generic_colors {
        pay_generic_spreading_colors(pool, generic, generic_order);
    } else {
        pay_generic_in_order(pool, generic, generic_order);
    }
}

pub(super) fn add_generic(mut cost: ManaCost, additional: u16) -> ManaCost {
    cost.generic = cost.generic.saturating_add(additional);
    cost
}

/// A cost reduction only ever removes generic mana, and never takes a cost
/// below its colored requirements (CR 601.2f).
pub(super) fn reduce_generic(mut cost: ManaCost, reduction: u16) -> ManaCost {
    cost.generic = cost.generic.saturating_sub(reduction);
    cost
}

pub(super) fn add_mana_cost(mut cost: ManaCost, additional: ManaCost) -> ManaCost {
    cost.generic = cost.generic.saturating_add(additional.generic);
    cost.white = cost.white.saturating_add(additional.white);
    cost.blue = cost.blue.saturating_add(additional.blue);
    cost.black = cost.black.saturating_add(additional.black);
    cost.red = cost.red.saturating_add(additional.red);
    cost.green = cost.green.saturating_add(additional.green);
    cost.colorless = cost.colorless.saturating_add(additional.colorless);
    for index in 0..HybridPair::COUNT {
        cost.hybrid[index] = cost.hybrid[index].saturating_add(additional.hybrid[index]);
    }
    cost.variable_x |= additional.variable_x;
    cost.x_multiplier = cost.x_multiplier.saturating_add(additional.x_multiplier);
    cost
}

/// Pays a generic requirement one mana at a time, cycling through the order
/// rather than draining each colour before touching the next. This is the
/// payment converge wants: with white, white, and blue in the pool, "{1}{W}"
/// spends one of each colour rather than both whites, which is what any
/// caster of a converge spell means by it.
pub(super) fn pay_generic_spreading_colors(pool: &mut ManaPool, amount: u16, order: &[ManaColor]) {
    let mut remaining = amount;
    while remaining > 0 {
        let mut spent_this_pass = false;
        for color in order {
            if remaining == 0 {
                break;
            }
            if pool.amount(*color) > 0 {
                pool.remove_color(*color, 1);
                remaining -= 1;
                spent_this_pass = true;
            }
        }
        if !spent_this_pass {
            break;
        }
    }
    debug_assert_eq!(remaining, 0);
}

pub(super) fn pay_generic_in_order(pool: &mut ManaPool, amount: u16, order: &[ManaColor]) {
    let mut remaining = amount;
    for color in order {
        let spent = pool.amount(*color).min(remaining);
        pool.remove_color(*color, spent);
        remaining -= spent;
        if remaining == 0 {
            break;
        }
    }
    debug_assert_eq!(remaining, 0);
}

pub(super) fn colored_mana() -> Vec<ManaColor> {
    vec![
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ]
}

/// "Spend only black mana on X." The restriction does not change how much the
/// spell costs, only which mana may pay for it, so folding the X portion out
/// of the generic requirement and into the coloured one says exactly that in
/// the vocabulary every payment path already speaks.
pub(super) fn fold_restricted_x(cost: ManaCost, x: u16, color: ManaColor) -> (ManaCost, u16) {
    let amount = x.saturating_mul(cost.x_multiplier);
    let mut folded = cost;
    match color {
        ManaColor::White => folded.white = folded.white.saturating_add(amount),
        ManaColor::Blue => folded.blue = folded.blue.saturating_add(amount),
        ManaColor::Black => folded.black = folded.black.saturating_add(amount),
        ManaColor::Red => folded.red = folded.red.saturating_add(amount),
        ManaColor::Green => folded.green = folded.green.saturating_add(amount),
        // No printed card restricts X to colourless, and generic already
        // accepts it, so there is nothing to fold.
        ManaColor::Colorless => return (cost, x),
    }
    (folded, 0)
}

pub(super) const fn mana_cost_amount(cost: ManaCost, color: ManaColor) -> u16 {
    match color {
        ManaColor::White => cost.white,
        ManaColor::Blue => cost.blue,
        ManaColor::Black => cost.black,
        ManaColor::Red => cost.red,
        ManaColor::Green => cost.green,
        // `{C}` is a requirement like any coloured symbol: only colorless
        // mana pays it. Generic is the part any mana pays, and is separate.
        ManaColor::Colorless => cost.colorless,
    }
}

/// Every symbol that has to be paid with particular mana rather than with
/// whatever is around: the five colours, `{C}`, and the hybrids.
pub(super) const fn colored_cost_total(cost: ManaCost) -> u16 {
    cost.white
        + cost.blue
        + cost.black
        + cost.red
        + cost.green
        + cost.colorless
        + cost.hybrid_total()
}

pub(super) const fn mana_cost_value(cost: ManaCost) -> u16 {
    cost.generic.saturating_add(colored_cost_total(cost))
}

/// How much of a hybrid pair's colours is left once the cost's own coloured
/// symbols are covered.
pub(super) fn available_hybrid(pool: ManaPool, cost: ManaCost, pair: HybridPair) -> u16 {
    let (first, second) = pair.colors();
    let spare = |color: ManaColor| {
        pool.amount(color)
            .saturating_sub(mana_cost_amount(cost, color))
    };
    spare(first).saturating_add(spare(second))
}

/// Whether one colour can pay any hybrid symbol this cost carries.
pub(super) fn hybrid_pays_with(cost: ManaCost, color: ManaColor) -> bool {
    HybridPair::ALL
        .into_iter()
        .any(|pair| cost.hybrid[pair.index()] > 0 && pair.contains(color))
}
