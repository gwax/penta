// @ts-check

/** @typedef {import("./game-types").Action} Action */

/**
 * @typedef MulliganBottomPicker
 * @property {Action[]} actions Complete backend actions represented by this picker.
 * @property {number[]} candidateCardIds Hand cards the player may select.
 * @property {number} required Number of cards the final selection must contain.
 */

/**
 * Whether an action is one complete London-mulligan bottom choice.
 *
 * @param {Action} action
 */
export function isMulliganBottomAction(action) {
  return action.bottomCardIds.length > 0;
}

/** @param {number[]} cards */
const selectionKey = (cards) => [...cards].sort((left, right) => left - right).join(",");

/**
 * Collapses the backend's complete legal bottom actions into one UI picker.
 * Returns null for absent or inconsistent action data so the caller can fall
 * back to rendering the original actions instead of hiding a legal choice.
 *
 * @param {Action[]} actions
 * @returns {MulliganBottomPicker | null}
 */
export function buildMulliganBottomPicker(actions) {
  const bottomActions = actions.filter(isMulliganBottomAction);
  if (bottomActions.length === 0) return null;

  const required = bottomActions[0].bottomCardIds.length;
  const seenSelections = new Set();
  for (const action of bottomActions) {
    const cards = action.bottomCardIds;
    const key = selectionKey(cards);
    if (
      cards.length !== required ||
      new Set(cards).size !== cards.length ||
      seenSelections.has(key)
    ) {
      return null;
    }
    seenSelections.add(key);
  }

  return {
    actions: bottomActions,
    candidateCardIds: Array.from(
      new Set(bottomActions.flatMap((action) => action.bottomCardIds)),
    ),
    required,
  };
}

/**
 * Applies one reversible picker click without committing anything to the
 * engine. Invalid candidates and selections beyond the exact bound are ignored.
 *
 * @param {MulliganBottomPicker | null} picker
 * @param {number[]} selection
 * @param {number} cardId
 */
export function toggleMulliganBottomCard(picker, selection, cardId) {
  if (!picker?.candidateCardIds.includes(cardId)) return selection;
  if (selection.includes(cardId)) {
    return selection.filter((candidate) => candidate !== cardId);
  }
  if (selection.length >= picker.required) return selection;
  const proposed = [...selection, cardId];
  const canComplete = picker.actions.some((action) =>
    proposed.every((candidate) => action.bottomCardIds.includes(candidate)),
  );
  return canComplete ? proposed : selection;
}

/**
 * Resolves a staged, order-independent UI selection back to the one complete
 * action the backend already declared legal.
 *
 * @param {MulliganBottomPicker | null} picker
 * @param {number[]} selection
 * @returns {Action | undefined}
 */
export function resolveMulliganBottomAction(picker, selection) {
  if (
    !picker ||
    selection.length !== picker.required ||
    new Set(selection).size !== selection.length
  ) {
    return undefined;
  }
  const key = selectionKey(selection);
  return picker.actions.find((action) => selectionKey(action.bottomCardIds) === key);
}
