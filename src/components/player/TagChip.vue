<script setup lang="ts">
import type { PlayerTag, TagTone } from "../../api";

defineProps<{ tag: PlayerTag }>();

const TONE: Record<TagTone, string> = {
  positive: "bg-emerald-900/70 text-emerald-200",
  negative: "bg-red-900/70 text-red-200",
  warning: "bg-amber-900/70 text-amber-200",
  neutral: "bg-zinc-700/80 text-zinc-200",
};
</script>

<template>
  <span class="group relative inline-flex">
    <span
      class="rounded px-1.5 py-0.5 text-[11px] leading-none whitespace-nowrap"
      :class="[TONE[tag.tone], tag.lowConfidence && 'opacity-60']"
    >
      {{ tag.label }}
    </span>
    <span
      class="pointer-events-none absolute top-full left-0 z-20 mt-1 hidden w-64 rounded-md border border-zinc-700 bg-zinc-950 p-2 text-xs leading-5 text-zinc-300 shadow-lg group-hover:block"
    >
      {{ tag.detail }}
      <span v-if="tag.lowConfidence" class="mt-1 block text-zinc-500">样本较少，仅供参考。</span>
    </span>
  </span>
</template>
