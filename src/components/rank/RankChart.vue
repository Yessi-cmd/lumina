<script setup lang="ts">
import { computed } from "vue";
import type { RankPoint } from "../../api";
import { useGameDataStore } from "../../stores/gameData";
import { rankScore, scoreLabel, signed, tierText } from "../../utils/rank";

/** One queue's points, oldest first. */
const props = defineProps<{ points: RankPoint[] }>();
const gd = useGameDataStore();

const WIDTH = 600;
const HEIGHT = 160;
const PAD_Y = 12;
const LINE = "#d97706";
/** Above this many points the dots would merge into a smear, so only the line is drawn. */
const MAX_DOTS = 80;

const scored = computed(() =>
  props.points.flatMap((p, index) => {
    const score = rankScore(p);
    return score === null ? [] : [{ ...p, score, index }];
  }),
);

const bounds = computed(() => {
  const scores = scored.value.map((p) => p.score);
  if (!scores.length) return { min: 0, max: 100 };
  const min = Math.min(...scores);
  const max = Math.max(...scores);
  const pad = Math.max(30, (max - min) * 0.1);
  return { min: Math.max(0, min - pad), max: max + pad };
});

function x(i: number): number {
  return (i / Math.max(1, scored.value.length - 1)) * WIDTH;
}

function y(score: number): number {
  const { min, max } = bounds.value;
  return PAD_Y + (1 - (score - min) / (max - min)) * (HEIGHT - PAD_Y * 2);
}

/** Division boundaries inside the range; tier boundaries only when the range is wide. */
const ticks = computed(() => {
  const { min, max } = bounds.value;
  const step = max - min > 600 ? 400 : 100;
  const out: number[] = [];
  for (let s = Math.ceil(min / step) * step; s <= max; s += step) out.push(s);
  return out;
});

const path = computed(() =>
  scored.value
    .map((p, i) => `${i === 0 ? "M" : "L"}${x(i).toFixed(1)},${y(p.score).toFixed(1)}`)
    .join(" "),
);
const area = computed(() => {
  if (!scored.value.length) return "";
  const last = x(scored.value.length - 1).toFixed(1);
  return `${path.value} L${last},${HEIGHT} L0,${HEIGHT} Z`;
});

function percent(value: number, total: number): string {
  return `${(value / total) * 100}%`;
}

function dotColor(outcome: RankPoint["outcome"]): string {
  if (outcome === "win") return "bg-emerald-400";
  if (outcome === "loss") return "bg-red-400";
  return "bg-zinc-400";
}

function date(ms: number): string {
  const d = new Date(ms);
  return `${d.getMonth() + 1}月${d.getDate()}日`;
}

function tip(p: (typeof scored.value)[number]) {
  const result = p.outcome === "win" ? "胜利" : p.outcome === "loss" ? "失败" : "变动";
  const change = p.delta === null ? "" : ` ${signed(p.delta)}`;
  let body: string | undefined;
  if (p.championId !== null) {
    const kda = p.kda ? ` · ${p.kda[0]}/${p.kda[1]}/${p.kda[2]}` : "";
    body = `${gd.championName(p.championId)}${kda}`;
  }
  return {
    title: `${date(p.at)} · ${result}${change}`,
    subtitle: `${tierText(p.tier, p.division)} ${p.lp} 点`,
    body,
  };
}
</script>

<template>
  <div class="pl-20">
    <div class="relative">
      <svg :viewBox="`0 0 ${WIDTH} ${HEIGHT}`" preserveAspectRatio="none" class="h-40 w-full overflow-visible">
        <line
          v-for="s in ticks"
          :key="s"
          x1="0"
          :x2="WIDTH"
          :y1="y(s)"
          :y2="y(s)"
          :stroke="s % 400 === 0 ? 'rgb(255 255 255 / 0.14)' : 'rgb(255 255 255 / 0.05)'"
          stroke-width="1"
          vector-effect="non-scaling-stroke"
        />
        <path :d="area" :fill="LINE" fill-opacity="0.1" />
        <path
          class="rank-line"
          :d="path"
          fill="none"
          :stroke="LINE"
          stroke-width="2"
          stroke-linejoin="round"
          stroke-linecap="round"
          vector-effect="non-scaling-stroke"
        />
      </svg>
      <span
        v-for="s in ticks"
        :key="s"
        class="pointer-events-none absolute -left-20 w-[4.5rem] -translate-y-1/2 text-right text-[10px] text-zinc-500"
        :style="{ top: percent(y(s), HEIGHT) }"
      >
        {{ scoreLabel(s) }}
      </span>
      <template v-if="scored.length <= MAX_DOTS">
        <span
          v-for="(p, i) in scored"
          :key="p.at"
          class="pointer-events-none absolute size-1.5 -translate-x-1/2 -translate-y-1/2 rounded-full ring-2 ring-zinc-950"
          :class="dotColor(p.outcome)"
          :style="{ left: percent(x(i), WIDTH), top: percent(y(p.score), HEIGHT) }"
        />
      </template>
      <!-- One hover column per point. -->
      <div class="absolute inset-0 flex">
        <div
          v-for="p in scored"
          :key="p.at"
          v-tip="tip(p)"
          class="h-full flex-1 hover:bg-white/[0.04]"
        />
      </div>
    </div>
    <div class="mt-2 flex justify-between text-[10px] text-zinc-500">
      <span>{{ scored.length ? date(scored[0].at) : "" }}</span>
      <span>绿点胜 · 红点负 · 灰点为无法判断的变动（如多局合并）</span>
      <span>{{ scored.length ? date(scored[scored.length - 1].at) : "" }}</span>
    </div>
  </div>
</template>

<style scoped>
.rank-line {
  stroke-dasharray: 4000;
  stroke-dashoffset: 4000;
  animation: draw 1.1s var(--ease-out-expo) forwards;
}
@keyframes draw {
  to {
    stroke-dashoffset: 0;
  }
}
</style>
