<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from "vue";
import { useSettingsStore } from "../../stores/settings";

const store = useSettingsStore();
const now = ref(Date.now());
let timer: ReturnType<typeof setInterval> | undefined;

watch(
  () => store.pendingAccept,
  (pending) => {
    clearInterval(timer);
    timer = undefined;
    if (pending) {
      now.value = Date.now();
      timer = setInterval(() => (now.value = Date.now()), 200);
    }
  },
  { immediate: true },
);
onUnmounted(() => clearInterval(timer));

const seconds = computed(() => {
  const pending = store.pendingAccept;
  return pending ? Math.max(0, Math.ceil((pending.acceptAt - now.value) / 1000)) : 0;
});
</script>

<template>
  <div
    v-if="store.pendingAccept"
    class="flex items-center gap-3 border-b border-amber-700/60 bg-amber-950/60 px-4 py-2 text-sm"
  >
    <span class="font-medium text-amber-200">找到对局</span>
    <span class="text-amber-100/80">
      {{ seconds > 0 ? `${seconds} 秒后自动接受` : "正在接受…" }}
    </span>
    <button
      class="ml-auto rounded-md border border-amber-600/70 px-3 py-1 text-amber-100 hover:bg-amber-900/60"
      @click="store.cancelAutoAccept()"
    >
      取消本次
    </button>
  </div>
</template>
