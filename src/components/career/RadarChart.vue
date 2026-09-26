<script setup lang="ts">
import { computed } from "vue";

export interface RadarAxis {
  label: string;
  /** 0–100; 50 is the reference. */
  score: number;
  tip: string;
}

const props = defineProps<{ axes: RadarAxis[]; referenceLabel: string }>();

// Validated pair for the dark surface (dataviz validator: lightness, CVD, contrast).
const ME = "#d97706";
const REFERENCE = "#0b8fd0";
const SIZE = 260;
const CENTER = SIZE / 2;
const RADIUS = 92;
const RINGS = [25, 50, 75, 100];

function point(index: number, score: number): [number, number] {
  const angle = (Math.PI * 2 * index) / props.axes.length - Math.PI / 2;
  const r = (Math.max(0, Math.min(100, score)) / 100) * RADIUS;
  return [CENTER + Math.cos(angle) * r, CENTER + Math.sin(angle) * r];
}

function polygon(scores: number[]): string {
  return scores.map((s, i) => point(i, s).join(",")).join(" ");
}

const mine = computed(() => polygon(props.axes.map((a) => a.score)));
const reference = computed(() => polygon(props.axes.map(() => 50)));
const labels = computed(() =>
  props.axes.map((axis, i) => {
    const [x, y] = point(i, 122);
    const anchor = Math.abs(x - CENTER) < 4 ? "middle" : x > CENTER ? "start" : "end";
    return { ...axis, x, y, anchor };
  }),
);
</script>

<template>
  <div class="flex flex-col items-center">
    <svg :viewBox="`0 0 ${SIZE} ${SIZE}`" class="w-full max-w-[340px] overflow-visible">
      <!-- Grid: rings at 25/50/75/100 and spokes, recessive. -->
      <polygon
        v-for="ring in RINGS"
        :key="ring"
        :points="polygon(axes.map(() => ring))"
        fill="none"
        :stroke="ring === 50 ? 'rgb(255 255 255 / 0.14)' : 'rgb(255 255 255 / 0.06)'"
        stroke-width="1"
      />
      <line
        v-for="(_, i) in axes"
        :key="`spoke-${i}`"
        :x1="CENTER"
        :y1="CENTER"
        :x2="point(i, 100)[0]"
        :y2="point(i, 100)[1]"
        stroke="rgb(255 255 255 / 0.06)"
        stroke-width="1"
      />

      <polygon :points="reference" fill="none" :stroke="REFERENCE" stroke-width="2" stroke-linejoin="round" />
      <polygon
        class="radar-shape"
        :points="mine"
        :fill="ME"
        fill-opacity="0.12"
        :stroke="ME"
        stroke-width="2"
        stroke-linejoin="round"
      />
      <g v-for="(axis, i) in axes" :key="`dot-${i}`" v-tip="{ title: axis.label, body: axis.tip }" class="cursor-help">
        <!-- Hit target larger than the mark. -->
        <circle :cx="point(i, axis.score)[0]" :cy="point(i, axis.score)[1]" r="12" fill="transparent" />
        <circle
          :cx="point(i, axis.score)[0]"
          :cy="point(i, axis.score)[1]"
          r="4"
          :fill="ME"
          stroke="var(--color-zinc-900)"
          stroke-width="2"
        />
      </g>

      <text
        v-for="l in labels"
        :key="l.label"
        :x="l.x"
        :y="l.y"
        :text-anchor="l.anchor"
        dominant-baseline="middle"
        class="fill-zinc-300 text-[12px]"
      >
        {{ l.label }}
        <tspan class="fill-zinc-500 text-[11px]"> {{ Math.round(l.score) }}</tspan>
      </text>
    </svg>

    <div class="mt-2 flex items-center gap-4 text-xs text-zinc-400">
      <span class="flex items-center gap-1.5">
        <span class="h-0.5 w-4 rounded-full" :style="{ background: ME }" />我
      </span>
      <span class="flex items-center gap-1.5">
        <span class="h-0.5 w-4 rounded-full" :style="{ background: REFERENCE }" />{{ referenceLabel }}（= 50）
      </span>
    </div>
  </div>
</template>

<style scoped>
.radar-shape {
  transform-origin: 50% 50%;
  animation: radar-in 0.6s var(--ease-out-expo);
}
@keyframes radar-in {
  from {
    transform: scale(0.6);
    opacity: 0;
  }
}
</style>
