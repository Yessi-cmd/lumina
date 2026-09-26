<script setup lang="ts">
import { computed } from "vue";
import type { Career } from "../../api";

const props = defineProps<{ trend: Career["trend"] }>();

/** Games per rolling window. */
const WINDOW = 10;
const WIDTH = 600;
const HEIGHT = 120;
const PAD_Y = 8;
const LINE = "#d97706";

const points = computed(() => {
  const list = props.trend;
  return list.map((game, i) => {
    const window = list.slice(Math.max(0, i - WINDOW + 1), i + 1);
    const wins = window.filter((g) => g.result === "win").length;
    return { ...game, rate: wins / window.length, index: i };
  });
});

function x(i: number): number {
  const n = Math.max(1, points.value.length - 1);
  return (i / n) * WIDTH;
}

function y(rate: number): number {
  return PAD_Y + (1 - rate) * (HEIGHT - PAD_Y * 2);
}

const path = computed(() =>
  points.value.map((p, i) => `${i === 0 ? "M" : "L"}${x(i).toFixed(1)},${y(p.rate).toFixed(1)}`).join(" "),
);
const area = computed(() => {
  if (points.value.length === 0) return "";
  const last = x(points.value.length - 1).toFixed(1);
  return `${path.value} L${last},${HEIGHT} L0,${HEIGHT} Z`;
});
const columnWidth = computed(() => WIDTH / Math.max(1, points.value.length));

function date(ms: number): string {
  const d = new Date(ms);
  return `${d.getMonth() + 1}月${d.getDate()}日`;
}
</script>

<template>
  <div>
    <div class="relative">
      <svg :viewBox="`0 0 ${WIDTH} ${HEIGHT}`" preserveAspectRatio="none" class="h-28 w-full overflow-visible">
        <line
          v-for="level in [0.25, 0.5, 0.75]"
          :key="level"
          x1="0"
          :x2="WIDTH"
          :y1="y(level)"
          :y2="y(level)"
          :stroke="level === 0.5 ? 'rgb(255 255 255 / 0.14)' : 'rgb(255 255 255 / 0.05)'"
          stroke-width="1"
          vector-effect="non-scaling-stroke"
        />
        <path :d="area" :fill="LINE" fill-opacity="0.1" />
        <path
          class="trend-line"
          :d="path"
          fill="none"
          :stroke="LINE"
          stroke-width="2"
          stroke-linejoin="round"
          stroke-linecap="round"
          vector-effect="non-scaling-stroke"
        />
      </svg>
      <div class="pointer-events-none absolute inset-y-0 -left-9 flex flex-col justify-between py-1 text-[10px] text-zinc-500">
        <span>100%</span><span>50%</span><span>0%</span>
      </div>
      <!-- One hover column per game, wider than the line itself. -->
      <div class="absolute inset-0 flex">
        <div
          v-for="p in points"
          :key="p.index"
          v-tip="{
            title: `${date(p.createdAt)} · ${p.result === 'win' ? '胜利' : '失败'}`,
            subtitle: `KDA ${p.kda.toFixed(2)}`,
            body: `近 ${Math.min(WINDOW, p.index + 1)} 场胜率 ${Math.round(p.rate * 100)}%`,
          }"
          class="h-full hover:bg-white/[0.04]"
          :style="{ width: `${columnWidth}px`, flex: '1 1 0' }"
        />
      </div>
    </div>
    <div class="mt-2 flex gap-[2px]">
      <span
        v-for="p in points"
        :key="p.index"
        class="h-2 flex-1 rounded-[2px]"
        :class="p.result === 'win' ? 'bg-emerald-500/80' : 'bg-red-500/70'"
      />
    </div>
    <div class="mt-1 flex justify-between text-[10px] text-zinc-500">
      <span>{{ points.length ? date(points[0].createdAt) : "" }}</span>
      <span>胜负（绿胜红负）· 曲线为近 {{ WINDOW }} 场滚动胜率</span>
      <span>{{ points.length ? date(points[points.length - 1].createdAt) : "" }}</span>
    </div>
  </div>
</template>

<style scoped>
.trend-line {
  stroke-dasharray: 2000;
  stroke-dashoffset: 2000;
  animation: draw 1.1s var(--ease-out-expo) forwards;
}
@keyframes draw {
  to {
    stroke-dashoffset: 0;
  }
}
</style>
