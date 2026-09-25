<script setup lang="ts">
import { onMounted } from "vue";
import { useAppStore } from "./stores/app";

const app = useAppStore();

const navItems = [
  { to: "/", label: "概览" },
  { to: "/match-history", label: "战绩" },
  { to: "/ongoing-game", label: "对局" },
  { to: "/settings", label: "设置" },
];

onMounted(() => {
  app.load().catch((err) => console.error("Failed to load app info", err));
});
</script>

<template>
  <div class="flex h-full">
    <aside class="flex w-44 shrink-0 flex-col border-r border-zinc-800 bg-zinc-900 p-3">
      <div class="mb-6 px-2 text-lg font-semibold tracking-wide text-amber-300">Lumina</div>
      <nav class="flex flex-col gap-1">
        <RouterLink
          v-for="item in navItems"
          :key="item.to"
          :to="item.to"
          class="rounded-md px-3 py-2 text-sm text-zinc-300 hover:bg-zinc-800"
          active-class="bg-zinc-800 text-white"
        >
          {{ item.label }}
        </RouterLink>
      </nav>
      <div class="mt-auto px-2 text-xs text-zinc-500">v{{ app.info?.version ?? "-" }}</div>
    </aside>
    <main class="min-w-0 flex-1 overflow-auto p-6">
      <RouterView />
    </main>
  </div>
</template>
