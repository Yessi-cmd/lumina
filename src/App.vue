<script setup lang="ts">
import { onMounted, watch } from "vue";
import { useRouter } from "vue-router";
import AutoAcceptBanner from "./components/common/AutoAcceptBanner.vue";
import { useAppStore } from "./stores/app";
import { useGameDataStore } from "./stores/gameData";
import { phaseLabel, useLcuStore } from "./stores/lcu";
import { useOngoingStore } from "./stores/ongoing";
import { useSettingsStore } from "./stores/settings";

const app = useAppStore();
const lcu = useLcuStore();
// Created here so game data loads as soon as the client connects.
useGameDataStore();
const ongoing = useOngoingStore();
const settings = useSettingsStore();
const router = useRouter();

// Jump to the game panel when champ select starts and again when the game loads.
watch(
  () => lcu.snapshot.gameflowPhase,
  (phase, previous) => {
    if (phase !== previous && (phase === "ChampSelect" || phase === "GameStart")) {
      router.push("/ongoing-game");
    }
  },
);

const navItems = [
  { to: "/", label: "概览" },
  { to: "/match-history", label: "战绩" },
  { to: "/ongoing-game", label: "对局" },
  { to: "/settings", label: "设置" },
];

onMounted(() => {
  app.load().catch((err) => console.error("Failed to load app info", err));
  lcu.start().catch((err) => console.error("Failed to start LCU store", err));
  ongoing.start().catch((err) => console.error("Failed to start ongoing store", err));
  settings.start().catch((err) => console.error("Failed to start settings store", err));
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
      <div class="mt-auto flex items-center gap-2 px-2 text-xs text-zinc-400">
        <span
          class="size-2 rounded-full"
          :class="lcu.connected ? 'bg-emerald-400' : 'bg-zinc-600'"
        />
        {{ lcu.connected ? phaseLabel(lcu.snapshot.gameflowPhase) : "未连接" }}
      </div>
      <div class="mt-2 px-2 text-xs text-zinc-500">v{{ app.info?.version ?? "-" }}</div>
    </aside>
    <div class="flex min-w-0 flex-1 flex-col">
      <AutoAcceptBanner />
      <main class="min-w-0 flex-1 overflow-auto p-6">
        <RouterView />
      </main>
    </div>
  </div>
</template>
