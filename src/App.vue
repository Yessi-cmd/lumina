<script setup lang="ts">
import { computed, onMounted, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import AppIcon, { type IconName } from "./components/common/AppIcon.vue";
import AutoAcceptBanner from "./components/common/AutoAcceptBanner.vue";
import UpdateBanner from "./components/common/UpdateBanner.vue";
import TitleBar from "./components/common/TitleBar.vue";
import TooltipLayer from "./components/common/TooltipLayer.vue";
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
  app.watchUpdates();
  lcu.start().catch((err) => console.error("Failed to start LCU store", err));
  ongoing.start().catch((err) => console.error("Failed to start ongoing store", err));
  settings.start().catch((err) => console.error("Failed to start settings store", err));
});
</script>

<template>
  <div class="relative isolate flex h-full">
    <!-- Aurora: soft colour fields drifting behind everything. Gradients rather than
         filter blur, so the motion stays a cheap compositor transform. -->
    <div class="pointer-events-none fixed inset-0 -z-10 overflow-hidden" aria-hidden="true">
      <div
        class="animate-aurora-a absolute -top-[30%] -left-[15%] h-[75%] w-[60%] rounded-full bg-[radial-gradient(closest-side,rgb(245_158_11/0.13),transparent)] will-change-transform"
      />
      <div
        class="animate-aurora-b absolute -top-[25%] right-[-20%] h-[70%] w-[55%] rounded-full bg-[radial-gradient(closest-side,rgb(56_189_248/0.09),transparent)] will-change-transform"
      />
      <div
        class="animate-aurora-c absolute right-[5%] -bottom-[35%] h-[70%] w-[55%] rounded-full bg-[radial-gradient(closest-side,rgb(139_92_246/0.08),transparent)] will-change-transform"
      />
      <!-- Fine grid, fading out from the top. -->
      <div
        class="absolute inset-0 bg-[linear-gradient(rgb(255_255_255/0.025)_1px,transparent_1px),linear-gradient(90deg,rgb(255_255_255/0.025)_1px,transparent_1px)] bg-[size:44px_44px] [mask-image:radial-gradient(ellipse_80%_60%_at_50%_0%,black,transparent)]"
      />
    </div>

    <aside
      class="relative flex w-56 shrink-0 flex-col bg-zinc-950/45 px-3 pb-3 backdrop-blur-xl select-none"
    >
      <span
        class="pointer-events-none absolute inset-y-0 right-0 w-px bg-linear-to-b from-white/[0.02] via-white/[0.08] to-white/[0.02]"
      />
      <div data-tauri-drag-region class="flex h-16 items-center gap-3 px-2">
        <div class="group/logo relative size-9 shrink-0">
          <!-- A lit rim turns slowly around the badge. -->
          <div
            class="animate-spin-slow absolute -inset-[3px] rounded-[13px] bg-[conic-gradient(from_0deg,transparent_0deg,rgb(252_211_77/0.9)_60deg,transparent_140deg,transparent_220deg,rgb(249_115_22/0.8)_280deg,transparent_360deg)] opacity-70 blur-[2px]"
          />
          <div
            class="relative flex size-9 items-center justify-center rounded-[10px] bg-linear-to-br from-amber-300 via-amber-400 to-orange-500 text-zinc-950 shadow-[inset_0_1px_0_rgb(255_255_255/0.5),0_8px_22px_-6px_rgb(245_158_11/0.8)] transition-transform duration-500 ease-spring group-hover/logo:scale-110 group-hover/logo:rotate-12"
          >
            <AppIcon name="sparkle" :size="18" :stroke-width="2.2" />
          </div>
        </div>
        <div data-tauri-drag-region class="leading-tight">
          <div
            class="bg-linear-to-r from-amber-100 via-white to-amber-200 bg-clip-text text-[16px] font-semibold tracking-tight text-transparent"
          >
            Lumina
          </div>
          <div class="text-[11px] text-zinc-500">v{{ app.info?.version ?? "-" }}</div>
        </div>
      </div>

      <div class="mt-3 px-3 pb-1.5 text-[10px] font-semibold tracking-[0.14em] text-zinc-600 uppercase">Menu</div>
      <nav class="relative flex flex-col gap-0.5">
        <div
          class="pointer-events-none absolute inset-x-0 top-0 h-9 rounded-lg bg-linear-to-r from-amber-400/[0.14] via-white/[0.05] to-white/[0.02] shadow-[inset_0_1px_0_0_rgb(255_255_255/0.06),0_4px_16px_-8px_rgb(245_158_11/0.4)] ring-1 ring-white/[0.06] transition-[transform,opacity] duration-500 ease-spring ring-inset"
          :class="activeIndex < 0 && 'opacity-0'"
          :style="{ transform: `translateY(${Math.max(activeIndex, 0) * NAV_PITCH}px)` }"
        >
          <span
            class="absolute top-1/2 left-0 h-5 w-[3px] -translate-y-1/2 rounded-full bg-linear-to-b from-amber-200 to-amber-500 shadow-[0_0_12px_2px_rgb(245_158_11/0.6)]"
          />
        </div>
        <RouterLink
          v-for="item in navItems"
          :key="item.to"
          v-slot="{ isExactActive, navigate, href }"
          :to="item.to"
          custom
        >
          <a
            :href="href"
            class="group/nav relative flex h-9 items-center gap-3 rounded-lg px-3 text-sm transition-colors duration-200"
            :class="isExactActive ? 'text-zinc-50' : 'text-zinc-400 hover:bg-white/[0.03] hover:text-zinc-100'"
            @click="navigate"
          >
            <AppIcon
              :name="item.icon"
              :size="17"
              class="transition-[transform,color] duration-300 ease-spring group-hover/nav:scale-115 group-active/nav:scale-95"
              :class="isExactActive && 'text-amber-300 drop-shadow-[0_0_6px_rgb(245_158_11/0.6)]'"
            />
            <span class="transition-transform duration-300 ease-out-expo group-hover/nav:translate-x-0.5">
              {{ item.label }}
            </span>
          </a>
        </RouterLink>
      </nav>

      <div
        class="mt-auto flex items-center gap-2.5 rounded-xl border border-white/[0.06] bg-linear-to-br from-white/[0.05] to-white/[0.01] p-2.5 shadow-[inset_0_1px_0_0_rgb(255_255_255/0.04)]"
      >
        <div class="relative shrink-0">
          <img
            v-if="summoner"
            :src="profileIconUrl(summoner.profileIconId)"
            class="size-9 rounded-full bg-zinc-800 ring-2 ring-amber-400/30"
          />
          <div v-else class="flex size-9 items-center justify-center rounded-full bg-zinc-800 text-zinc-500">
            <AppIcon name="user" :size="16" />
          </div>
          <span
            v-if="lcu.connected"
            class="absolute -right-0.5 -bottom-0.5 size-2.5 animate-ping rounded-full bg-emerald-400/60"
          />
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
      <UpdateBanner />
      <main class="min-w-0 flex-1 overflow-auto px-6 pt-2 pb-8">
        <RouterView v-slot="{ Component, route: current }">
          <Transition name="page" mode="out-in">
            <component :is="Component" :key="current.path" />
          </Transition>
        </RouterView>
      </main>
    </div>
    <TooltipLayer />
  </div>
</template>
