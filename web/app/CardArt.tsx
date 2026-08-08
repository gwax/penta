"use client";

import { isScryfallId, type CardArtMode } from "./card-art-mode";

export type { CardArtMode } from "./card-art-mode";

const scryfallCroppedArtUrl = (id: string) =>
  `https://cards.scryfall.io/art/front/${id[0]}/${id[1]}/${id}.webp`;

const scryfallFullArtUrl = (id: string, size: "normal" | "large") =>
  `https://cards.scryfall.io/${size}/front/${id[0]}/${id[1]}/${id}.jpg`;

export function CardArt({
  mode = "off",
  cardKind,
  scryfallId,
  fullImageSizes = "132px",
  onImageError,
}: {
  mode?: CardArtMode;
  cardKind: string;
  scryfallId: string;
  fullImageSizes?: string;
  onImageError?: () => void;
}) {
  const hasValidArt = mode !== "off" && isScryfallId(scryfallId);
  const image = hasValidArt
    ? mode === "cropped"
      ? { src: scryfallCroppedArtUrl(scryfallId) }
      : {
          src: scryfallFullArtUrl(scryfallId, "normal"),
          srcSet: `${scryfallFullArtUrl(scryfallId, "normal")} 488w, ${scryfallFullArtUrl(scryfallId, "large")} 672w`,
          sizes: fullImageSizes,
        }
    : null;

  return (
    <span
      className={mode === "full" && image ? "card-art card-art-full" : "card-art"}
      aria-hidden="true"
    >
      <i>{cardKind.includes("land") ? "▲" : cardKind.includes("artifact") ? "◇" : "●"}</i>
      {image && (
        // eslint-disable-next-line @next/next/no-img-element
        <img
          key={image.src}
          {...image}
          alt=""
          draggable={false}
          loading="lazy"
          decoding="async"
          // Scryfall needs the card's ID to serve the image; it does not need
          // to know which page asked, and a game URL carries the seed and both
          // decklists in its query string.
          referrerPolicy="no-referrer"
          onError={(event) => {
            event.currentTarget.hidden = true;
            onImageError?.();
          }}
        />
      )}
    </span>
  );
}
