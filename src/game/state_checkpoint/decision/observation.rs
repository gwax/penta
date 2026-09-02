// The pending decision itself, as a checkpoint sees it. A decision is
// written only for a seat entitled to see it: a private question belongs to
// its chooser, and a continuation naming an object this viewer cannot
// resolve is not written at all.

pub(super) fn decision_snapshot(
    game: &Game,
    viewer: PlayerId,
    pending: &PendingDecision,
) -> Option<DecisionStateSnapshot> {
    // A private decision is absent from this viewer's ordinary observation.
    // Serializing its continuation anyway would expose raw candidate ids and
    // effect-local bindings through the checkpoint, so fail reconstruction
    // closed for the non-choosing seat instead.
    if pending.observation.visibility == DecisionVisibility::Private
        && pending.observation.player != viewer
    {
        return None;
    }
    let card_origins = visible_decision_card_origins(game, viewer, pending);
    if decision_referenced_object_ids(&pending.continuation)
        .into_iter()
        .any(|object| {
            object_reference_requires_hidden_rebinding(game, viewer, object)
                && !card_origins
                    .iter()
                    .any(|origin| origin.object_id == object.0)
        })
    {
        return None;
    }
    let visible_rebindings = card_origins
        .iter()
        .map(|origin| GameObjectId(origin.object_id))
        .collect::<Vec<_>>();
    let options = pending
        .observation
        .options
        .iter()
        .map(|option| decision_option_snapshot(&game.catalog, option))
        .collect::<Option<Vec<_>>>()?;
    Some(DecisionStateSnapshot {
        preference: preference_snapshot(pending.observation.preference),
        options,
        card_origins,
        continuation: continuation_snapshot(
            game,
            viewer,
            &pending.continuation,
            &visible_rebindings,
        )?,
    })
}
