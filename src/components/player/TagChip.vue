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

// Shown by the app-wide tooltip, so only one is ever on screen.
const tip = computed(() => ({
  title: props.tag.label,
  subtitle: props.tag.lowConfidence ? "样本较少，仅供参考" : undefined,
  body: props.tag.detail,
}));
</script>

<template>
  <span
    v-tip="tip"
    class="cursor-default rounded-md px-1.5 py-[3px] text-[11px] leading-none font-medium whitespace-nowrap ring-1 transition-[filter] duration-150 ring-inset hover:brightness-125"
    :class="[TONE[tag.tone], tag.lowConfidence && 'opacity-60']"
  >
    {{ tag.label }}
  </span>
</template>
