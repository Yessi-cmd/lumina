import { reactive, type Directive } from "vue";

/** What a tooltip shows: plain text, or a title with optional subtitle and body. */
export interface TipContent {
  title: string;
  subtitle?: string;
  body?: string;
}
export type Tip = string | TipContent | null | undefined;

/** The one tooltip on screen, drawn by `TooltipLayer`. */
export const tooltip = reactive<{ content: TipContent | null; rect: DOMRect | null }>({
  content: null,
  rect: null,
});

const SHOW_DELAY = 90;
const tips = new WeakMap<HTMLElement, Tip>();
let owner: HTMLElement | null = null;
let timer: ReturnType<typeof setTimeout> | undefined;

function normalize(tip: Tip): TipContent | null {
  if (!tip) return null;
  return typeof tip === "string" ? { title: tip } : tip;
}

function show(el: HTMLElement) {
  const content = normalize(tips.get(el));
  if (!content) return;
  owner = el;
  clearTimeout(timer);
  // Moving from one tipped element to the next switches at once; a fresh hover waits
  // a moment so sweeping the mouse across a row does not flash tooltips.
  const delay = tooltip.content ? 0 : SHOW_DELAY;
  timer = setTimeout(() => {
    if (owner !== el) return;
    tooltip.rect = el.getBoundingClientRect();
    tooltip.content = content;
  }, delay);
}

function hide(el: HTMLElement) {
  if (owner !== el) return;
  owner = null;
  clearTimeout(timer);
  timer = setTimeout(() => {
    if (!owner) tooltip.content = null;
  }, 60);
}

const enter = (e: Event) => show(e.currentTarget as HTMLElement);
const leave = (e: Event) => hide(e.currentTarget as HTMLElement);

/** `v-tip="'text'"` or `v-tip="{ title, subtitle, body }"`. */
export const vTip: Directive<HTMLElement, Tip> = {
  mounted(el, binding) {
    tips.set(el, binding.value);
    el.addEventListener("mouseenter", enter);
    el.addEventListener("mouseleave", leave);
  },
  updated(el, binding) {
    tips.set(el, binding.value);
    if (owner === el) tooltip.content = normalize(binding.value);
  },
  unmounted(el) {
    el.removeEventListener("mouseenter", enter);
    el.removeEventListener("mouseleave", leave);
    if (owner === el) {
      owner = null;
      tooltip.content = null;
    }
  },
};

declare module "vue" {
  interface GlobalDirectives {
    vTip: typeof vTip;
  }
}
