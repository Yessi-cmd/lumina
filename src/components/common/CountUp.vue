<script setup lang="ts">
import { onUnmounted, ref, watch } from "vue";

// A number that counts to its value (from 0 on first show, from the old value after).
const props = withDefaults(defineProps<{ value: number; decimals?: number; duration?: number }>(), {
  decimals: 0,
  duration: 900,
});

const shown = ref(0);
let frame = 0;

watch(
  () => props.value,
  (to) => {
    cancelAnimationFrame(frame);
    const from = shown.value;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches || !Number.isFinite(to)) {
      shown.value = to;
      return;
    }
    const start = performance.now();
    const tick = (now: number) => {
      const t = Math.min(1, (now - start) / props.duration);
      // Exponential ease-out, matching --ease-out-expo.
      const eased = t === 1 ? 1 : 1 - Math.pow(2, -10 * t);
      shown.value = from + (to - from) * eased;
      if (t < 1) frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
  },
  { immediate: true },
);
onUnmounted(() => cancelAnimationFrame(frame));
</script>

<template>
  <span class="tabular-nums">{{ shown.toFixed(decimals) }}</span>
</template>
