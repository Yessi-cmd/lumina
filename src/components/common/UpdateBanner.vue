<script setup lang="ts">
import { computed } from "vue";
import { useAppStore } from "../../stores/app";

const app = useAppStore();
const shown = computed(() => {
  const u = app.update;
  return u?.available && app.dismissed !== u.latest ? u : null;
});
</script>

<template>
  <Transition
    enter-from-class="-translate-y-3 opacity-0"
    leave-to-class="-translate-y-2 opacity-0"
    enter-active-class="transition-all duration-500 ease-spring"
    leave-active-class="transition-all duration-150"
  >
    <div
      v-if="shown"
      class="relative mx-6 mb-2 flex items-center gap-3 rounded-xl border border-sky-400/25 bg-linear-to-r from-sky-500/15 via-sky-500/[0.06] to-transparent px-4 py-2 text-sm backdrop-blur-md"
    >
      <span class="relative flex size-2">
        <span class="absolute inline-flex size-full animate-ping rounded-full bg-sky-400 opacity-60" />
        <span class="relative inline-flex size-2 rounded-full bg-sky-400" />
      </span>
      <span class="font-semibold text-sky-200">发现新版本 v{{ shown.latest }}</span>
      <span class="text-xs text-zinc-400">当前 v{{ shown.current }}</span>
      <button class="btn btn-primary ml-auto py-1" @click="app.openRelease()">前往下载</button>
      <button class="btn btn-ghost py-1" v-tip="'这个版本不再提示'" @click="app.dismissUpdate()">忽略</button>
    </div>
  </Transition>
</template>
