<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import { tooltip } from "./tooltip";

const GAP = 8;
const MARGIN = 8;
const box = ref<HTMLElement | null>(null);
const position = ref({ left: 0, top: 0 });
/** Hidden until measured; glides between elements only once it is already showing. */
const placed = ref(false);
const glide = ref(false);

// Below the element and centred on it; above when there is no room below; always on
// screen horizontally. Measured after render, since the size depends on the content.
watch(
  () => [tooltip.content, tooltip.rect] as const,
  async ([content, rect], [previous]) => {
    if (!content || !rect) return;
    // A fresh tooltip is measured unseen; one moving to a new element glides there.
    glide.value = !!previous;
    if (!previous) placed.value = false;
    await nextTick();
    const el = box.value;
    if (!el) return;
    const { width, height } = el.getBoundingClientRect();
    const center = rect.left + rect.width / 2;
    const left = Math.min(Math.max(center - width / 2, MARGIN), window.innerWidth - width - MARGIN);
    const fitsBelow = rect.bottom + GAP + height <= window.innerHeight - MARGIN;
    const top = fitsBelow ? rect.bottom + GAP : rect.top - GAP - height;
    position.value = { left, top };
    placed.value = true;
  },
);
</script>

<template>
  <Transition name="tip">
    <div
      v-if="tooltip.content"
      ref="box"
      class="pointer-events-none fixed top-0 left-0 z-50 max-w-72"
      :class="glide && 'transition-transform duration-200 ease-out-expo'"
      :style="{
        transform: `translate(${position.left}px, ${position.top}px)`,
        visibility: placed ? 'visible' : 'hidden',
      }"
    >
      <!-- Inner box: the outer one's transform is taken by the position. -->
      <div
        class="animate-pop-in rounded-lg border border-white/10 bg-zinc-900/85 px-3 py-2 text-xs shadow-[inset_0_1px_0_0_rgb(255_255_255/0.06),0_18px_40px_-12px_rgb(0_0_0/0.8)] backdrop-blur-xl"
      >
      <div class="font-medium text-zinc-50">{{ tooltip.content.title }}</div>
      <div v-if="tooltip.content.subtitle" class="mt-0.5 text-amber-300/90">{{ tooltip.content.subtitle }}</div>
      <div
        v-if="tooltip.content.body"
        class="mt-1.5 border-t border-white/[0.06] pt-1.5 leading-5 whitespace-pre-line text-zinc-400"
      >
        {{ tooltip.content.body }}
      </div>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.tip-enter-active {
  transition: opacity 0.15s ease-out;
}
.tip-leave-active {
  transition: opacity 0.1s ease-in;
}
.tip-enter-from,
.tip-leave-to {
  opacity: 0;
}
</style>
