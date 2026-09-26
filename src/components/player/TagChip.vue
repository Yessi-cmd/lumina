<script setup lang="ts">
import type { PlayerTag, TagTone } from "../../api";

defineProps<{ tag: PlayerTag }>();

const TONE: Record<TagTone, string> = {
  positive: "bg-emerald-400/10 text-emerald-300 ring-emerald-400/25",
  negative: "bg-red-400/10 text-red-300 ring-red-400/25",
  warning: "bg-amber-400/10 text-amber-300 ring-amber-400/25",
  neutral: "bg-white/5 text-zinc-300 ring-white/10",
};
</script>

<template>
  <span class="group relative inline-flex">
    <span
      class="rounded-md px-1.5 py-[3px] text-[11px] leading-none font-medium whitespace-nowrap ring-1 ring-inset"
      :class="[TONE[tag.tone], tag.lowConfidence && 'opacity-60']"
    >
      {{ tag.label }}
    </span>
    <span
      class="pointer-events-none absolute top-full left-0 z-20 mt-1.5 hidden w-64 animate-fade-in rounded-lg border border-white/10 bg-zinc-900/95 p-2.5 text-xs leading-5 text-zinc-300 shadow-xl shadow-black/40 backdrop-blur group-hover:block"
    >
      {{ tag.detail }}
      <span v-if="tag.lowConfidence" class="mt-1 block text-zinc-500">样本较少，仅供参考。</span>
    </span>
  </span>
</template>
