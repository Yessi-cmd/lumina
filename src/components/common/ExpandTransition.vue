<script setup lang="ts">
// Height animation for content shown with v-if. CSS cannot transition to `height: auto`,
// so the height is measured, animated in pixels, and released once the animation ends.
const DURATION = "0.38s";
const EASING = "var(--ease-out-expo)";

function animate(el: HTMLElement, from: string, to: string, opacity: string, done: () => void) {
  el.style.overflow = "hidden";
  el.style.height = from;
  // Commit the start height before transitioning away from it.
  void el.offsetHeight;
  el.style.transition = `height ${DURATION} ${EASING}, opacity ${DURATION} ${EASING}`;
  el.style.height = to;
  el.style.opacity = opacity;
  const finish = (event: TransitionEvent) => {
    if (event.target !== el || event.propertyName !== "height") return;
    el.removeEventListener("transitionend", finish);
    done();
  };
  el.addEventListener("transitionend", finish);
}

function onEnter(element: Element, done: () => void) {
  const el = element as HTMLElement;
  el.style.opacity = "0";
  animate(el, "0px", `${el.scrollHeight}px`, "1", done);
}

function onAfterEnter(element: Element) {
  const el = element as HTMLElement;
  // Back to natural height, so content that loads later can still grow it.
  el.style.height = "";
  el.style.overflow = "";
  el.style.transition = "";
}

function onLeave(element: Element, done: () => void) {
  const el = element as HTMLElement;
  animate(el, `${el.scrollHeight}px`, "0px", "0", done);
}
</script>

<template>
  <Transition :css="false" @enter="onEnter" @after-enter="onAfterEnter" @leave="onLeave">
    <slot />
  </Transition>
</template>
