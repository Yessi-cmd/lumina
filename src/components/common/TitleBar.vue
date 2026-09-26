<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import { onMounted, onUnmounted, ref } from "vue";
import AppIcon from "./AppIcon.vue";

defineProps<{ title?: string }>();

// The window is undecorated (tauri.conf.json), so the title bar and its buttons live here.
const win = getCurrentWindow();
const maximized = ref(false);
let unlisten: (() => void) | undefined;

async function syncMaximized() {
  maximized.value = await win.isMaximized();
}

onMounted(async () => {
  syncMaximized().catch(() => {});
  unlisten = await win.onResized(() => {
    syncMaximized().catch(() => {});
  });
});
onUnmounted(() => unlisten?.());
</script>

<template>
  <header data-tauri-drag-region class="flex h-10 shrink-0 items-center pl-6 select-none">
    <span data-tauri-drag-region class="truncate text-xs text-zinc-500">{{ title }}</span>
    <div data-tauri-drag-region class="h-full flex-1" />
    <div class="flex h-full">
      <button class="titlebar-button" title="最小化" @click="win.minimize()">
        <AppIcon name="minimize" :size="15" />
      </button>
      <button class="titlebar-button" :title="maximized ? '还原' : '最大化'" @click="win.toggleMaximize()">
        <svg
          v-if="maximized"
          viewBox="0 0 24 24"
          width="13"
          height="13"
          fill="none"
          stroke="currentColor"
          stroke-width="1.8"
          stroke-linejoin="round"
        >
          <rect x="4" y="8" width="12" height="12" rx="2" />
          <path d="M8 8V6a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2h-2" />
        </svg>
        <AppIcon v-else name="maximize" :size="13" />
      </button>
      <button class="titlebar-button hover:bg-red-600! hover:text-white!" title="关闭" @click="win.close()">
        <AppIcon name="close" :size="15" />
      </button>
    </div>
  </header>
</template>

<style scoped>
@reference "../../style.css";

.titlebar-button {
  @apply flex h-full w-11 cursor-pointer items-center justify-center text-zinc-400 transition-colors hover:bg-white/[0.07] hover:text-zinc-100;
}
</style>
