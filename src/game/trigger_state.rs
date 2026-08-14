use std::borrow::Cow;

use crate::action::{AbilityOrigin, Target};
use crate::card::{
    AbilityTargetDef, CardSupertype, CardTypeSet, EffectDef, PlayerRelation, TriggerConditionDef,
    TriggerEventDef, TurnStepDef, ZoneKind,
};
use crate::casting::TargetSelection;
use crate::ids::{
    CardDefinitionId, ChoiceIndex, GameObjectId, ObjectBindingIndex, ObjectSetBindingIndex,
    PlayerId,
};

use super::{ScopedEffect, StackAbilityResolver, StackObject};

/// An effect queued for the next time a step begins. Whatever queued it has
/// usually left by then, so the entry carries its own source and controller.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DelayedTrigger {
    /// The object that queued this, kept whole so the effect resolves with
    /// the same source and controller it would have had at the time.
    pub(super) object: Box<StackObject>,
    /// Trigger-event information and effect-local bindings captured when the
    /// effect was scheduled.
    pub(super) context: EffectResolutionContext,
    pub(super) step: TurnStepDef,
    pub(super) player: PlayerRelation,
    pub(super) effect: ScopedEffect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct TriggerContext {
    pub(super) object: Option<GameObjectId>,
    pub(super) object_controller: Option<PlayerId>,
    pub(super) event_player: Option<PlayerId>,
    pub(super) amount: Option<i32>,
}

impl TriggerContext {
    pub(super) const fn empty() -> Self {
        Self {
            object: None,
            object_controller: None,
            event_player: None,
            amount: None,
        }
    }
}

/// State local to one declarative effect resolution. Trigger information is
/// kept separate and copyable because it is also captured by abilities before
/// they ever resolve; bindings belong only to a particular continuation of an
/// effect program.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct EffectResolutionContext {
    pub(super) trigger: TriggerContext,
    single_objects: [Option<Target>; ObjectBindingIndex::COUNT],
    object_groups: [Vec<Target>; ObjectSetBindingIndex::COUNT],
}

impl EffectResolutionContext {
    pub(super) fn new(trigger: TriggerContext) -> Self {
        Self {
            trigger,
            single_objects: [None; ObjectBindingIndex::COUNT],
            object_groups: std::array::from_fn(|_| Vec::new()),
        }
    }

    pub(super) fn empty() -> Self {
        Self::new(TriggerContext::empty())
    }

    pub(super) const fn single_object(&self, binding: ObjectBindingIndex) -> Option<Target> {
        self.single_objects[binding.index()]
    }

    pub(super) fn bind_single_object(
        &mut self,
        binding: ObjectBindingIndex,
        object: Option<Target>,
    ) {
        self.single_objects[binding.index()] = object;
    }

    pub(super) fn object_group(&self, binding: ObjectSetBindingIndex) -> &[Target] {
        &self.object_groups[binding.index()]
    }

    pub(super) fn bind_object_group(
        &mut self,
        binding: ObjectSetBindingIndex,
        objects: Vec<Target>,
    ) {
        self.object_groups[binding.index()] = objects;
    }

    pub(super) fn single_objects(&self) -> &[Option<Target>; ObjectBindingIndex::COUNT] {
        &self.single_objects
    }

    pub(super) fn object_groups(&self) -> &[Vec<Target>; ObjectSetBindingIndex::COUNT] {
        &self.object_groups
    }

    /// Compatibility bridge for the current choice-bearing effect variants.
    /// Their authored [`ChoiceIndex`] is replaced by `ObjectBindingIndex` in
    /// the next model migration; keeping that translation here avoids mixing
    /// the two concepts in stored resolution state.
    pub(super) fn single_object_for_choice(&self, choice: ChoiceIndex) -> Option<Target> {
        ObjectBindingIndex::from_index(choice.index())
            .and_then(|binding| self.single_object(binding))
    }

    pub(super) fn bind_single_object_for_choice(
        &mut self,
        choice: ChoiceIndex,
        object: Option<Target>,
    ) {
        if let Some(binding) = ObjectBindingIndex::from_index(choice.index()) {
            self.bind_single_object(binding, object);
        }
    }

    pub(super) fn from_bindings(
        trigger: TriggerContext,
        single_objects: [Option<Target>; ObjectBindingIndex::COUNT],
        object_groups: [Vec<Target>; ObjectSetBindingIndex::COUNT],
    ) -> Self {
        Self {
            trigger,
            single_objects,
            object_groups,
        }
    }
}

impl From<TriggerContext> for EffectResolutionContext {
    fn from(trigger: TriggerContext) -> Self {
        Self::new(trigger)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub(super) struct TriggerEventObject {
    pub(super) id: GameObjectId,
    pub(super) token: bool,
    pub(super) types: CardTypeSet,
    pub(super) controller: PlayerId,
    pub(super) colors: [bool; 5],
    pub(super) subtypes: Cow<'static, [&'static str]>,
    pub(super) mana_value: u16,
    /// Current power where one exists: a battlefield creature reports what it
    /// is now, not what it was printed as.
    pub(super) power: Option<i16>,
    /// Current toughness, read the same way and with the same caveat.
    pub(super) toughness: Option<i16>,
    pub(super) supertypes: [bool; CardSupertype::COUNT],
    /// Whether this object is in combat. Cheap to carry and it cannot feed
    /// back into a characteristic, unlike a keyword or a static bonus.
    pub(super) attacking_or_blocking: bool,
    /// The object's keywords, as a bitmask over
    /// [`crate::card::KeywordAbility::simple_index`].
    ///
    /// Unlike power, this includes keywords a static continuous effect grants
    /// or removes: `Game::keyword_mask` stratifies the layer-6 walk rather than
    /// omitting it, so a predicate here and the combat rules give one answer.
    /// The one exception is the walk's own recipient matching, which reads the
    /// layer below itself; `Game::collect_ability_layer_operations` says why.
    pub(super) keywords: u32,
    /// Whether this creature is attacking, excluding a creature that is only
    /// blocking. Bloodrush and similar predicates need the narrower state.
    pub(super) attacking: bool,
    /// Whether the object is a tapped permanent. Cheap to carry, and like
    /// `attacking` it cannot feed back into a characteristic.
    pub(super) tapped: bool,
    /// Whether this creature attacked at any point this turn, which outlives
    /// combat and so is not the same question as `attacking`.
    pub(super) attacked_this_turn: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum CommittedTriggerEvent {
    ZoneChanged {
        object: TriggerEventObject,
        from: ZoneKind,
        to: ZoneKind,
    },
    BecomesTapped {
        object: TriggerEventObject,
    },
    LifeGained {
        player: PlayerId,
        amount: u16,
    },
    AttacksInGroup {
        object: TriggerEventObject,
        total: u8,
    },
    Attacks {
        object: TriggerEventObject,
    },
    BecomesBlocked {
        object: TriggerEventObject,
        /// Blockers beyond the first, so a clause reading the trigger amount
        /// gets the quantity it is printed against without recounting.
        blockers_beyond_first: u16,
    },
    /// An attacker that no creature blocked, committed once blockers are
    /// declared.
    AttacksAndIsNotBlocked {
        object: TriggerEventObject,
    },
    /// One side of one blocking relationship. Emitted once per ordered pair,
    /// so a clause on either creature sees the other as the triggering
    /// object without having to know which of them attacked.
    BlocksOrBecomesBlocked {
        creature: TriggerEventObject,
        other: TriggerEventObject,
    },
    TappedForMana {
        object: TriggerEventObject,
    },
    DamageDealt {
        source: TriggerEventObject,
        recipient: Target,
        amount: u16,
        combat: bool,
    },
    CombatDamageDealtToPlayer {
        object: TriggerEventObject,
        player: PlayerId,
        amount: u16,
    },
    DamageDealtToPlayer {
        object: TriggerEventObject,
        player: PlayerId,
        amount: u16,
    },
    SpellCast {
        object: TriggerEventObject,
    },
    Transformed {
        object: TriggerEventObject,
    },
    StepBegins {
        step: TurnStepDef,
        player: PlayerId,
    },
    DamagedCreatureDied {
        object: TriggerEventObject,
        source: GameObjectId,
    },
}

impl CommittedTriggerEvent {
    pub(super) fn context(&self) -> TriggerContext {
        match self {
            Self::ZoneChanged { object, .. }
            | Self::BecomesTapped { object }
            | Self::AttacksInGroup { object, .. }
            | Self::Attacks { object }
            | Self::Transformed { object }
            | Self::DamagedCreatureDied { object, .. }
            | Self::AttacksAndIsNotBlocked { object } => TriggerContext {
                object: Some(object.id),
                object_controller: Some(object.controller),
                event_player: None,
                amount: None,
            },
            Self::DamageDealt {
                source,
                recipient,
                amount,
                ..
            } => TriggerContext {
                object: Some(source.id),
                object_controller: Some(source.controller),
                event_player: match recipient {
                    Target::Player(player) => Some(*player),
                    Target::Card(_) | Target::Permanent(_) | Target::Spell(_) => None,
                },
                amount: Some(i32::from(*amount)),
            },
            Self::CombatDamageDealtToPlayer {
                object,
                player,
                amount,
            }
            | Self::DamageDealtToPlayer {
                object,
                player,
                amount,
            } => TriggerContext {
                object: Some(object.id),
                object_controller: Some(object.controller),
                event_player: Some(*player),
                amount: Some(i32::from(*amount)),
            },
            Self::BlocksOrBecomesBlocked { other, .. } => TriggerContext {
                object: Some(other.id),
                object_controller: Some(other.controller),
                event_player: None,
                amount: None,
            },
            Self::BecomesBlocked {
                object,
                blockers_beyond_first,
            } => TriggerContext {
                object: Some(object.id),
                object_controller: Some(object.controller),
                event_player: None,
                amount: Some(i32::from(*blockers_beyond_first)),
            },
            Self::LifeGained { player, amount } => TriggerContext {
                object: None,
                object_controller: None,
                event_player: Some(*player),
                amount: Some(i32::from(*amount)),
            },
            // The player who tapped a permanent for mana is its controller,
            // which is the same shape a cast spell has.
            Self::TappedForMana { object } | Self::SpellCast { object } => TriggerContext {
                object: Some(object.id),
                object_controller: Some(object.controller),
                event_player: Some(object.controller),
                amount: None,
            },
            Self::StepBegins { player, .. } => TriggerContext {
                object: None,
                object_controller: None,
                event_player: Some(*player),
                amount: None,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct AbilitySourceRef {
    pub(super) object: GameObjectId,
    pub(super) ability: AbilityOrigin,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PendingTrigger {
    pub(super) id: u32,
    pub(super) source: AbilitySourceRef,
    pub(super) definition: CardDefinitionId,
    pub(super) owner: PlayerId,
    pub(super) controller: PlayerId,
    pub(super) text: &'static str,
    pub(super) target_defs: &'static [AbilityTargetDef],
    pub(super) targets: Vec<TargetSelection>,
    pub(super) effect: EffectDef,
    pub(super) resolver: StackAbilityResolver,
    pub(super) context: TriggerContext,
    pub(super) condition: Option<&'static TriggerConditionDef>,
}

/// The immutable declaration captured when one event matches one source
/// ability. The game assigns the ephemeral trigger ID when it accepts this
/// record into the pending-trigger queue.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct TriggerCapture {
    pub(super) source: AbilitySourceRef,
    pub(super) definition: CardDefinitionId,
    pub(super) owner: PlayerId,
    pub(super) controller: PlayerId,
    pub(super) text: &'static str,
    pub(super) target_defs: &'static [AbilityTargetDef],
    pub(super) effect: EffectDef,
    pub(super) resolver: StackAbilityResolver,
    pub(super) context: TriggerContext,
    /// The intervening-if condition this trigger reads, checked both when the
    /// ability would go on the stack and again when it resolves.
    pub(super) condition: Option<&'static TriggerConditionDef>,
}

/// A triggered ability with no object behind it, installed by an effect and
/// listening until its controller's next turn begins. Everything the trigger
/// needs is frozen here, because the ability that created it has finished
/// resolving and its source may be long gone.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct FloatingTrigger {
    pub(super) event: TriggerEventDef,
    pub(super) capture: TriggerCapture,
    pub(super) until_turn_of: PlayerId,
    /// How many turns that player had already started, so the turn the
    /// ability resolved during does not count as their next one.
    pub(super) created_after_turns: u32,
}

/// One battlefield trigger listener frozen at the start of an atomic event.
/// A simultaneous zone change can remove the source before another object in
/// the same event is published, so listener discovery cannot consult the
/// incrementally-mutated battlefield.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct BattlefieldTriggerListener {
    pub(super) event: TriggerEventDef,
    pub(super) uses_stack: bool,
    pub(super) capture: TriggerCapture,
}

#[derive(Clone, Debug)]
pub(super) struct TriggerPlacementBatch {
    pub(super) controller: PlayerId,
    pub(super) triggers: Vec<PendingTrigger>,
}
