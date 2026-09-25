<script setup lang="ts">
import { computed } from "vue";
import { useSettingsStore } from "../stores/settings";

const store = useSettingsStore();
const s = computed(() => store.settings);

function onDelayInput(event: Event) {
  const value = Number((event.target as HTMLInputElement).value);
  store.update({ autoAcceptDelaySecs: value });
}
</script>

<template>
  <section class="flex max-w-2xl flex-col gap-4">
    <h1 class="text-xl font-semibold">设置</h1>

    <p v-if="!s" class="text-sm text-zinc-400">加载中…</p>

    <div v-else class="rounded-lg border border-zinc-800 bg-zinc-900 p-4">
      <label class="flex cursor-pointer items-center justify-between gap-4">
        <div>
          <div class="font-medium">自动接受对局</div>
          <div class="text-sm text-zinc-400">匹配成功后自动点击接受。倒计时期间可以在顶部横幅取消。</div>
        </div>
        <input
          type="checkbox"
          class="size-5 accent-amber-500"
          :checked="s.autoAccept"
          @change="store.update({ autoAccept: ($event.target as HTMLInputElement).checked })"
        />
      </label>

      <div class="mt-4 flex items-center gap-4" :class="!s.autoAccept && 'opacity-50'">
        <span class="w-20 shrink-0 text-sm text-zinc-300">延迟</span>
        <input
          type="range"
          min="0"
          max="10"
          step="1"
          class="flex-1 accent-amber-500"
          :disabled="!s.autoAccept"
          :value="s.autoAcceptDelaySecs"
          @change="onDelayInput"
        />
        <span class="w-12 text-right text-sm tabular-nums">{{ s.autoAcceptDelaySecs }} 秒</span>
      </div>

      <p v-if="store.saveError" class="mt-3 text-sm text-red-400">{{ store.saveError }}</p>
    </div>
  </section>
</template>
