// Overlay rows line up with the five player slots of the client's champ-select screen,
// measured on a 720px-high client (the overlay is zoomed to the real client height).

export const ROW_HEIGHT = 74;
const FIRST_ROW_TOP = 100;
const ROW_PITCH = 80;

export function rowTop(index: number): number {
  return FIRST_ROW_TOP + index * ROW_PITCH;
}

/** Lane names as the Chinese client shows them. */
export const LANES: Record<string, string> = {
  TOP: "上路",
  JUNGLE: "打野",
  MIDDLE: "中路",
  BOTTOM: "下路",
  UTILITY: "辅助",
};
