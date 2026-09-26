<script setup lang="ts">
import { computed, onMounted, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import AppIcon, { type IconName } from "./components/common/AppIcon.vue";
import AutoAcceptBanner from "./components/common/AutoAcceptBanner.vue";
import TitleBar from "./components/common/TitleBar.vue";
import { useAppStore } from "./stores/app";
import { profileIconUrl, useGameDataStore } from "./stores/gameData";
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
const route = useRoute();

// Jump to the game panel when champ select starts and again when the game loads.
watch(
  () => lcu.snapshot.gameflowPhase,
  (phase, previous) => {
    if (phase !== previous && (phase === "ChampSelect" || phase === "GameStart")) {
      router.push("/ongoing-game");
    }
  },
);

const navItems: { to: string; label: string; icon: IconName }[] = [
  { to: "/", label: "概览", icon: "dashboard" },
  { to: "/match-history", label: "战绩", icon: "history" },
  { to: "/ongoing-game", label: "对局", icon: "swords" },
  { to: "/settings", label: "设置", icon: "settings" },
];

const pageTitle = computed(() => navItems.find((i) => i.to === route.path)?.label ?? "");
/** Nav items are h-9 with a 2px gap; the highlight slides between them. */
const NAV_PITCH = 38;
const activeIndex = computed(() => navItems.findIndex((i) => i.to === route.path));

const summoner = computed(() => lcu.snapshot.summoner);
const summonerName = computed(() => {
  const s = summoner.value;
  if (!s) return "未登录";
  return s.gameName || s.displayName || "未登录";
});
const statusText = computed(() => {
  if (lcu.connected) return phaseLabel(lcu.snapshot.gameflowPhase);
  return lcu.snapshot.status === "connecting" ? "连接中…" : "未连接客户端";
});
const statusDot = computed(() => {
  if (lcu.connected) return "bg-emerald-400 shadow-[0_0_8px] shadow-emerald-400/70";
  if (lcu.snapshot.status === "connecting") return "animate-pulse bg-amber-400";
  return "bg-zinc-500";
});

onMounted(() => {
  app.load().catch((err) => console.error("Failed to load app info", err));
  lcu.start().catch((err) => console.error("Failed to start LCU store", err));
  ongoing.start().catch((err) => console.error("Failed to start ongoing store", err));
  settings.start().catch((err) => console.error("Failed to start settings store", err));
});
</script>

<template>
  <div class="flex h-full">
    <aside
      class="flex w-56 shrink-0 flex-col border-r border-white/[0.06] bg-zinc-950/50 px-3 pb-3 select-none"
    >
      <div data-tauri-drag-region class="flex h-16 items-center gap-2.5 px-2">
        <div
          class="flex size-8 items-center justify-center rounded-lg bg-linear-to-br from-amber-300 to-orange-500 text-zinc-950 shadow-[0_6px_18px_-6px_rgb(245_158_11/0.7)]"
        >
          <AppIcon name="sparkle" :size="17" :stroke-width="2.2" />
        </div>
        <div data-tauri-drag-region class="leading-tight">
          <div class="text-[15px] font-semibold tracking-tight text-zinc-50">Lumina</div>
          <div class="text-[11px] text-zinc-500">v{{ app.info?.version ?? "-" }}</div>
        </div>
      </div>

      <nav class="relative mt-2 flex flex-col gap-0.5">
        <div
          class="pointer-events-none absolute inset-x-0 top-0 h-9 rounded-lg bg-white/[0.07] transition-[transform,opacity] duration-500 ease-out-expo"
          :class="activeIndex < 0 && 'opacity-0'"
          :style="{ transform: `translateY(${Math.max(activeIndex, 0) * NAV_PITCH}px)` }"
        >
          <span class="absolute top-1/2 left-0 h-4 w-[3px] -translate-y-1/2 rounded-full bg-amber-400" />
        </div>
        <RouterLink
          v-for="item in navItems"
          :key="item.to"
          :to="item.to"
          class="relative flex h-9 items-center gap-3 rounded-lg px-3 text-sm text-zinc-400 transition-colors duration-200 hover:text-zinc-100"
          exact-active-class="text-zinc-50!"
        >
          <AppIcon :name="item.icon" :size="17" />
          {{ item.label }}
        </RouterLink>
      </nav>

      <div class="mt-auto flex items-center gap-2.5 rounded-xl border border-white/[0.06] bg-white/[0.03] p-2.5">
        <div class="relative shrink-0">
          <img
            v-if="summoner"
            :src="profileIconUrl(summoner.profileIconId)"
            class="size-9 rounded-full bg-zinc-800 ring-1 ring-white/10"
          />
          <div v-else class="flex size-9 items-center justify-center rounded-full bg-zinc-800 text-zinc-500">
            <AppIcon name="user" :size="16" />
          </div>
          <span
            class="absolute -right-0.5 -bottom-0.5 size-2.5 rounded-full ring-2 ring-zinc-950"
            :class="statusDot"
          />
        </div>
        <div class="min-w-0">
          <div class="truncate text-sm font-medium text-zinc-100">{{ summonerName }}</div>
          <div class="truncate text-xs text-zinc-500">{{ statusText }}</div>
        </div>
      </div>
    </aside>

    <div class="flex min-w-0 flex-1 flex-col">
      <TitleBar :title="pageTitle" />
      <AutoAcceptBanner />
      <main class="min-w-0 flex-1 overflow-auto px-6 pt-2 pb-8">
        <RouterView v-slot="{ Component, route: current }">
          <Transition name="page" mode="out-in">
            <component :is="Component" :key="current.path" />
          </Transition>
        </RouterView>
      </main>
    </div>
  </div>
</template>
