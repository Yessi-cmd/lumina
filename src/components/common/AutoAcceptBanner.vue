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
  <Transition
    enter-from-class="-translate-y-2 opacity-0"
    leave-to-class="-translate-y-2 opacity-0"
    enter-active-class="transition duration-200"
    leave-active-class="transition duration-150"
  >
    <div
      v-if="store.pendingAccept"
      class="mx-6 mb-2 flex items-center gap-3 rounded-xl border border-amber-400/25 bg-linear-to-r from-amber-500/15 to-amber-500/5 px-4 py-2.5 text-sm shadow-[0_8px_24px_-12px_rgb(245_158_11/0.5)]"
    >
      <span class="relative flex size-2.5">
        <span class="absolute inline-flex size-full animate-ping rounded-full bg-amber-400 opacity-60" />
        <span class="relative inline-flex size-2.5 rounded-full bg-amber-400" />
      </span>
      <span class="font-semibold text-amber-200">找到对局</span>
      <span class="text-amber-100/70 tabular-nums">
        {{ seconds > 0 ? `${seconds} 秒后自动接受` : "正在接受…" }}
      </span>
      <button class="btn btn-secondary ml-auto py-1" @click="store.cancelAutoAccept()">取消本次</button>
    </div>
  </Transition>
</template>
