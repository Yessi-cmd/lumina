<script setup lang="ts">
import { computed } from "vue";
import type { PlayerTag, TagTone } from "../../api";

const props = defineProps<{ tag: PlayerTag }>();

const TONE: Record<TagTone, string> = {
  positive: "bg-emerald-400/10 text-emerald-300 ring-emerald-400/25",
  negative: "bg-red-400/10 text-red-300 ring-red-400/25",
  warning: "bg-amber-400/10 text-amber-300 ring-amber-400/25",
  neutral: "bg-white/5 text-zinc-300 ring-white/10",
};

/** 小代 gets a burning chip of its own: gold on a teammate, red on an opponent. */
const carry = computed(() => props.tag.id === "carry");
const enemyCarry = computed(() => carry.value && props.tag.tone !== "positive");

// Shown by the app-wide tooltip, so only one is ever on screen.
const tip = computed(() => ({
  title: props.tag.label,
  subtitle: props.tag.lowConfidence ? "样本较少，仅供参考" : undefined,
  body: props.tag.detail,
}));
</script>

<template>
  <span
    v-if="carry"
    v-tip="tip"
    class="carry-chip relative inline-flex cursor-default items-center gap-0.5 overflow-hidden rounded-md py-[3px] pr-1.5 pl-1 text-[11px] leading-none font-bold whitespace-nowrap text-white"
    :class="enemyCarry ? 'carry-enemy' : 'carry-ally'"
  >
    <svg viewBox="0 0 24 24" class="carry-flame relative size-3" fill="currentColor" aria-hidden="true">
      <path
        d="M12 2c.6 3.2-1 5-2.6 6.8C7.8 10.6 6 12.6 6 15.6A6 6 0 0 0 12 22a6 6 0 0 0 6-6.2c0-2.6-1.3-4.4-2.6-6-.2 1.6-.9 2.8-2 3.4.4-3.6-.4-7.6-1.4-11.2Z"
      />
    </svg>
    <span class="relative tracking-wide">{{ tag.label }}</span>
  </span>
  <span
    v-else
    v-tip="tip"
    class="cursor-default rounded-md px-1.5 py-[3px] text-[11px] leading-none font-medium whitespace-nowrap ring-1 transition-[filter] duration-150 ring-inset hover:brightness-125"
    :class="[TONE[tag.tone], tag.lowConfidence && 'opacity-60']"
  >
    {{ tag.label }}
  </span>
</template>

<style scoped>
.carry-chip {
  background-size: 220% 100%;
  text-shadow: 0 1px 2px rgb(0 0 0 / 0.45);
  animation:
    carry-flow 2.4s linear infinite,
    carry-pulse 1.6s ease-in-out infinite;
}
.carry-ally {
  --carry-glow: rgb(251 146 60 / 0.75);
  background-image: linear-gradient(100deg, #f59e0b, #f97316, #ef4444, #f97316, #f59e0b);
}
.carry-enemy {
  --carry-glow: rgb(239 68 68 / 0.8);
  background-image: linear-gradient(100deg, #dc2626, #f43f5e, #c026d3, #f43f5e, #dc2626);
}
/* A light sweep across the chip. */
.carry-chip::after {
  content: "";
  position: absolute;
  inset: 0;
  background: linear-gradient(100deg, transparent 30%, rgb(255 255 255 / 0.55) 50%, transparent 70%);
  transform: translateX(-120%);
  animation: carry-sweep 2.4s ease-in-out infinite;
}
.carry-flame {
  filter: drop-shadow(0 0 3px rgb(255 237 213 / 0.9));
  animation: carry-flicker 0.9s ease-in-out infinite alternate;
  transform-origin: 50% 90%;
}
@keyframes carry-flow {
  to {
    background-position: 220% 0;
  }
}
@keyframes carry-pulse {
  0%,
  100% {
    box-shadow: 0 0 4px var(--carry-glow), inset 0 0 0 1px rgb(255 255 255 / 0.25);
  }
  50% {
    box-shadow: 0 0 12px var(--carry-glow), inset 0 0 0 1px rgb(255 255 255 / 0.4);
  }
}
@keyframes carry-sweep {
  0%,
  40% {
    transform: translateX(-120%);
  }
  100% {
    transform: translateX(120%);
  }
}
@keyframes carry-flicker {
  from {
    transform: scale(0.9) rotate(-4deg);
  }
  to {
    transform: scale(1.12) rotate(4deg);
  }
}
@media (prefers-reduced-motion: reduce) {
  .carry-chip,
  .carry-chip::after,
  .carry-flame {
    animation: none;
  }
}
</style>
