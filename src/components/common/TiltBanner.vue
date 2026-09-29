<script setup lang="ts">
import { useSettingsStore } from "../../stores/settings";

const store = useSettingsStore();
</script>

<template>
  <Transition
    enter-from-class="-translate-y-3 scale-[0.98] opacity-0 blur-sm"
    leave-to-class="-translate-y-2 opacity-0"
    enter-active-class="transition-all duration-500 ease-spring"
    leave-active-class="transition-all duration-150"
  >
    <div
      v-if="store.tilt"
      class="relative mx-6 mb-2 flex items-center gap-3 overflow-hidden rounded-xl border border-red-400/30 bg-linear-to-r from-red-500/20 via-red-500/10 to-orange-500/5 px-4 py-2.5 text-sm shadow-[0_8px_24px_-12px_rgb(239_68_68/0.5)] backdrop-blur-md"
    >
      <span class="font-semibold text-red-200">先歇一歇？</span>
      <span class="text-red-100/70">
        排位已连败 {{ store.tilt.losses }} 把，共 {{ store.tilt.lp }} 点。连败之后状态容易下滑，喝口水再打。
      </span>
      <button class="btn btn-secondary ml-auto shrink-0 py-1" @click="store.dismissTilt()">知道了</button>
    </div>
  </Transition>
</template>
