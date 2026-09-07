# Effect programs and local behavior

The [design doctrine](design-doctrine.md#ownership-of-behavior) separates core
semantics from ownership of compositions. The [card guide](implementing-cards.md)
keeps each card readable at its declaration. Declarative syntax is useful, but
is not a requirement to force every exceptional procedure into a universal
grammar or a card-shaped engine variant.

## Current authoring and execution

Rust functions, local values, branches, and loops may construct ordinary
effect, cost, and ability structures in their owning card or set module.
Runtime code currently interprets those structures; construction-time Rust is
not a new callback or serialization protocol. Keep one-use components inline
unless a coherent local procedure warrants the documented readability exception.

The first migration demonstrates several ownership levels:

- Common enters/dies triggers and scry remain in `card::abilities`.
- Battle cry belongs in Mirrodin Besieged and is imported by Modern Horizons.
- Battalion belongs in Gatecrash; mobilize belongs in Tarkir: Dragonstorm.
- Bloomburrow's forage helper composes a choice of ordinary exile and sacrifice
  costs. The core need not recognize a `Forage` cost variant.
- Endurance's graveyard instruction composes a target-relative collection,
  random ordering, and a move in its own declaration. The core need not know
  an Endurance-shaped `BuryGraveyard` operation.

Helper names and source locations are not runtime dispatch keys. Moving a
helper between ownership levels does not require adding a core operation.

## Contract for local runtime exceptions

A bounded runtime exception is a legitimate future extension. It need not be
generalized merely to claim declarative coverage. This migration does not add
that interface; introducing one requires an explicit design for:

- ordinary timing, targets, source identity, stack use, and ability granting;
- engine-mediated actions, replacement processing, and event ordering;
- allowed state queries, hidden information, and engine-owned seeded randomness;
- typed inputs, outputs, and explicit resumable continuation state;
- catalog validation, complete coverage, checkpoint/replay reconstruction, and
  any affected compatibility migration;
- a correct reference path independent of optional prepared compilation.

A local callback taking unrestricted mutable `Game` is not this contract.
Neither are independently maintained affordability and execution callbacks.
When a card overrides a shared rule, expose the override and its lifetime
explicitly rather than teaching unrelated engine paths the card's identity.

## Payment programs: next semantic slice

Keep one general cost grammar. Common payments and locally composed action
costs should use the same payment window, selections, and execution validator.
Alternatives, bundles, and repetitions are composition, not new card-specific
cost variants. Card-local behavior remains local, including Herald of
Leshrac's choice of land and control-changing action.

The intended lifecycle is selection, executable-plan validation, and commitment:

- Collect all resource selections for one payment, allowing edits or decline
  where the rules permit it. Do not enumerate every complete combination as a
  separate player-facing choice.
- Validate the combined projected state and legal action order. An object may
  participate in several compatible actions; a tap-for-mana and a convoke tap
  cannot both consume its one untapped state. Mana contributions remain
  distinct from actual mana production and spending.
- A ready plan fixes resource assignments and accounts for required follow-up
  decisions. Revalidate relevant state before commitment. A plan promises
  legal execution, not that replacements leave every intended outcome intact.
- Do not use visible mutation followed by general rollback as cancellation.
  Protect both players' information and the recorded random stream. Costs
  involving random or information-producing actions need explicit treatment;
  they are not permission to simulate hidden outcomes for the planner.
- Publish a typed payment result and continue the caller's procedure. Scope
  decline to the payment: declining cumulative upkeep does not undo its age
  counter. Its helper should eventually express the tagged upkeep trigger,
  counter addition, repeated payment, and unpaid consequence as a program.

The runtime payment plan is semantic state. It is distinct from the optional,
catalog-derived programs in `src/prepared_engine` and must work with prepared
execution disabled. This migration does not implement the payment window or
change the current cumulative-upkeep representation.

## Subsequent migrations

Audit engine procedures as three different categories: genuine primitives,
misplaced compositions, and legitimate local exceptions. Start from concrete
consumers and preserve their complete behavior; do not shrink enums as an end
in itself.

- Endure can compose an effect choice, counters, and token creation. Detain
  and unleash need ordinary duration/conditional rule compositions while
  preserving mechanic identity and ability-grant/removal behavior.
- Typed effect outcomes should let later steps read actual damage, discarded
  or destroyed objects, and zone successors without dedicated follow-up fields.
- Separate cast permissions from exile selection/movement for Crabomination
  and related cards; preserve group-wide cast limits and resolution timing.
- Give Grist a resumable local repetition program; give Doomsday a genuine
  multi-zone search and independently authored remainder handling.
- Separate voting from Council's Judgment's exile consequence. Conspiracy
  should own the voting composition, with card-specific outcomes.

Preserve observable mechanic identity and event boundaries when expanding a
keyword. Keep genuinely atomic operations, such as exchange and simultaneous
damage, atomic. Ordinary sequential effects do not acquire the payment
window's all-or-nothing guarantee.

## Prepared execution

The [prepared engine](prepared-engine.md) may recognize and fuse compositions
regardless of whether a common helper, a set helper, or a card authored them.
Preserve choices, events, identity, and continuation boundaries. Select a full
supported lowering or the reference implementation before mutation; local
programs must remain correct when no lowering exists. Do not put optimization
flags or prepared payloads in card declarations.
