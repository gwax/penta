// @ts-check

/** @typedef {import("./game-types").Card} Card */

/**
 * Public choices attached to a permanent must be part of its visual pile
 * identity. Otherwise two Needles naming different cards (or two Caverns
 * naming different creature types) collapse into one misleading pile.
 *
 * @param {Card} card
 */
export function cardChoiceStateKey(card) {
  return `${card.chosenCardName ?? ""}\u0000${card.chosenCreatureType ?? ""}`;
}

/** @param {Card} card */
export function cardChoiceLabel(card) {
  if (card.chosenCardName) return `Named card: ${card.chosenCardName}`;
  if (card.chosenCreatureType) return `Chosen type: ${card.chosenCreatureType}`;
  return null;
}
