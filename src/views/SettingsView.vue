<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { api } from "../api";
import AppIcon from "../components/common/AppIcon.vue";
import ToggleSwitch from "../components/common/ToggleSwitch.vue";
import { useSettingsStore } from "../stores/settings";

const store = useSettingsStore();
const s = computed(() => store.settings);

const logDir = ref<string | null>(null);
const logError = ref<string | null>(null);

onMounted(() => {
  api
    .logDir()
    .then((dir) => (logDir.value = dir))
    .catch((err) => (logError.value = String(err)));
});

function openLogDir() {
  logError.value = null;
  api.openLogDir().catch((err) => (logError.value = String(err)));
}

function onDelayInput(event: Event) {
  const value = Number((event.target as HTMLInputElement).value);
  store.update({ autoAcceptDelaySecs: value });
}
</script>

<template>
  <section class="flex max-w-3xl flex-col gap-5">
    <header>
      <div class="eyebrow">Settings</div>
      <h1 class="page-title mt-1">设置</h1>
    </header>

    <p v-if="!s" class="text-sm text-zinc-400">加载中…</p>

    <template v-else>
      <div>
        <h2 class="eyebrow mb-2 px-1">对局</h2>
        <div class="card divide-y divide-white/[0.05]">
          <div class="flex items-center justify-between gap-6 p-4">
            <div>
              <div class="font-medium text-zinc-100">自动接受对局</div>
              <div class="mt-0.5 text-sm text-zinc-400">匹配成功后自动点击接受。倒计时期间可以在顶部横幅取消。</div>
            </div>
            <ToggleSwitch :model-value="s.autoAccept" @update:model-value="store.update({ autoAccept: $event })" />
          </div>

          <div class="flex items-center gap-4 p-4" :class="!s.autoAccept && 'opacity-50'">
            <span class="w-20 shrink-0 text-sm text-zinc-300">接受延迟</span>
            <input
              type="range"
              min="0"
              max="10"
              step="1"
              class="flex-1 cursor-pointer accent-amber-500 disabled:cursor-not-allowed"
              :disabled="!s.autoAccept"
              :value="s.autoAcceptDelaySecs"
              @change="onDelayInput"
            />
            <span
              class="w-14 rounded-md bg-white/5 py-0.5 text-center text-sm text-zinc-200 tabular-nums"
            >
              {{ s.autoAcceptDelaySecs }} 秒
            </span>
          </div>

          <div class="flex items-center justify-between gap-6 p-4">
            <div>
              <div class="font-medium text-zinc-100">自动弹出对局面板</div>
              <div class="mt-0.5 text-sm text-zinc-400">
                进入英雄选择和加载界面时，把 Lumina 窗口切到前台并显示对局页。游戏若为独占全屏，弹出可能会让游戏最小化。
              </div>
            </div>
            <ToggleSwitch
              :model-value="s.autoShowPanel"
              @update:model-value="store.update({ autoShowPanel: $event })"
            />
          </div>
        </div>
      </div>

      <div>
        <h2 class="eyebrow mb-2 px-1">英雄数据</h2>
        <div class="card divide-y divide-white/[0.05]">
          <div class="flex items-center justify-between gap-6 p-4">
            <div>
              <div class="font-medium text-zinc-100">英雄数据分段</div>
              <div class="mt-0.5 text-sm text-zinc-400">选英雄助手的强度和出装按哪个分段统计（数据来自 lolalytics）。</div>
            </div>
            <select
              class="field w-36 shrink-0"
              :value="s.statsTier"
              @change="store.update({ statsTier: ($event.target as HTMLSelectElement).value })"
            >
              <option value="all">全分段</option>
              <option value="platinum_plus">铂金及以上</option>
              <option value="emerald_plus">翡翠及以上</option>
              <option value="diamond_plus">钻石及以上</option>
            </select>
          </div>

          <div class="flex items-center justify-between gap-6 p-4">
            <div>
              <div class="font-medium text-zinc-100">克制关系分段</div>
              <div class="mt-0.5 text-sm text-zinc-400">
                对位克制看高分段：双方都能把英雄玩到位时，对位差距才真实。样本少的对位不下结论。
              </div>
            </div>
            <select
              class="field w-36 shrink-0"
              :value="s.matchupTier"
              @change="store.update({ matchupTier: ($event.target as HTMLSelectElement).value })"
            >
              <option value="emerald_plus">翡翠及以上</option>
              <option value="diamond_plus">钻石及以上</option>
              <option value="master_plus">大师及以上</option>
            </select>
          </div>
        </div>
        <p v-if="store.saveError" class="mt-2 px-1 text-sm text-red-400">{{ store.saveError }}</p>
      </div>

      <div>
        <h2 class="eyebrow mb-2 px-1">诊断</h2>
        <div class="card flex items-center justify-between gap-6 p-4">
          <div class="min-w-0">
            <div class="font-medium text-zinc-100">日志</div>
            <div class="mt-0.5 text-sm text-zinc-400">
              运行记录自动写入日志文件（单个 5MB，保留最近 5 个）。遇到问题时把日志发给开发者。
            </div>
            <div
              v-if="logDir"
              class="mt-2 truncate rounded-md bg-zinc-950/60 px-2 py-1 font-mono text-xs text-zinc-500"
              :title="logDir"
            >
              {{ logDir }}
            </div>
          </div>
          <button class="btn btn-secondary shrink-0" @click="openLogDir">
            <AppIcon name="folder" :size="15" />
            打开日志目录
          </button>
        </div>
        <p v-if="logError" class="mt-2 px-1 text-sm text-red-400">{{ logError }}</p>
      </div>
    </template>
  </section>
</template>
