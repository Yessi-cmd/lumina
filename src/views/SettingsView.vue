<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { api } from "../api";
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
  <section class="flex max-w-2xl flex-col gap-4">
    <h1 class="text-xl font-semibold">设置</h1>

    <p v-if="!s" class="text-sm text-zinc-400">加载中…</p>

    <div v-else class="rounded-lg border border-zinc-800 bg-zinc-900 p-4">
      <label class="flex cursor-pointer items-center justify-between gap-4">
        <div>
          <div class="font-medium">自动接受对局</div>
          <div class="text-sm text-zinc-400">匹配成功后自动点击接受。倒计时期间可以在顶部横幅取消。</div>
        </div>
        <input
          type="checkbox"
          class="size-5 accent-amber-500"
          :checked="s.autoAccept"
          @change="store.update({ autoAccept: ($event.target as HTMLInputElement).checked })"
        />
      </label>

      <div class="mt-4 flex items-center gap-4" :class="!s.autoAccept && 'opacity-50'">
        <span class="w-20 shrink-0 text-sm text-zinc-300">延迟</span>
        <input
          type="range"
          min="0"
          max="10"
          step="1"
          class="flex-1 accent-amber-500"
          :disabled="!s.autoAccept"
          :value="s.autoAcceptDelaySecs"
          @change="onDelayInput"
        />
        <span class="w-12 text-right text-sm tabular-nums">{{ s.autoAcceptDelaySecs }} 秒</span>
      </div>

      <label class="mt-4 flex cursor-pointer items-center justify-between gap-4 border-t border-zinc-800 pt-4">
        <div>
          <div class="font-medium">自动弹出对局面板</div>
          <div class="text-sm text-zinc-400">
            进入英雄选择和加载界面时，把 Lumina 窗口切到前台并显示对局页。游戏若为独占全屏，弹出可能会让游戏最小化。
          </div>
        </div>
        <input
          type="checkbox"
          class="size-5 accent-amber-500"
          :checked="s.autoShowPanel"
          @change="store.update({ autoShowPanel: ($event.target as HTMLInputElement).checked })"
        />
      </label>

      <div class="mt-4 flex items-center justify-between gap-4 border-t border-zinc-800 pt-4">
        <div>
          <div class="font-medium">英雄数据分段</div>
          <div class="text-sm text-zinc-400">选英雄助手的强度和出装按哪个分段统计（数据来自 lolalytics）。</div>
        </div>
        <select
          class="rounded-md border border-zinc-700 bg-zinc-900 px-2 py-1 text-sm"
          :value="s.statsTier"
          @change="store.update({ statsTier: ($event.target as HTMLSelectElement).value })"
        >
          <option value="all">全分段</option>
          <option value="platinum_plus">铂金及以上</option>
          <option value="emerald_plus">翡翠及以上</option>
          <option value="diamond_plus">钻石及以上</option>
        </select>
      </div>

      <div class="mt-4 flex items-center justify-between gap-4 border-t border-zinc-800 pt-4">
        <div>
          <div class="font-medium">克制关系分段</div>
          <div class="text-sm text-zinc-400">
            对位克制看高分段：双方都能把英雄玩到位时，对位差距才真实。样本少的对位不下结论。
          </div>
        </div>
        <select
          class="rounded-md border border-zinc-700 bg-zinc-900 px-2 py-1 text-sm"
          :value="s.matchupTier"
          @change="store.update({ matchupTier: ($event.target as HTMLSelectElement).value })"
        >
          <option value="emerald_plus">翡翠及以上</option>
          <option value="diamond_plus">钻石及以上</option>
          <option value="master_plus">大师及以上</option>
        </select>
      </div>

      <p v-if="store.saveError" class="mt-3 text-sm text-red-400">{{ store.saveError }}</p>
    </div>

    <div v-if="s" class="rounded-lg border border-zinc-800 bg-zinc-900 p-4">
      <div class="font-medium">客户端目录</div>
      <div class="text-sm text-zinc-400">
        以管理员身份连接过一次后会自动记住，之后普通启动即可连接（国服客户端以管理员运行时需要）。
        也可以手动填写 LeagueClientUx.exe 所在的文件夹。
      </div>
      <input
        :value="s.clientDir"
        placeholder="例如 D:\WeGameApps\英雄联盟\LeagueClient"
        class="mt-2 w-full rounded-md border border-zinc-700 bg-zinc-950 px-3 py-1.5 font-mono text-sm outline-none focus:border-amber-400"
        @change="store.update({ clientDir: ($event.target as HTMLInputElement).value })"
      />
    </div>

    <div class="rounded-lg border border-zinc-800 bg-zinc-900 p-4">
      <div class="flex items-center justify-between gap-4">
        <div class="min-w-0">
          <div class="font-medium">日志</div>
          <div class="text-sm text-zinc-400">
            运行记录自动写入日志文件（单个 5MB，保留最近 5 个）。遇到问题时把日志发给开发者。
          </div>
          <div v-if="logDir" class="mt-1 truncate font-mono text-xs text-zinc-500" :title="logDir">
            {{ logDir }}
          </div>
        </div>
        <button
          class="shrink-0 rounded-md border border-zinc-700 px-3 py-1.5 text-sm hover:bg-zinc-800"
          @click="openLogDir"
        >
          打开日志目录
        </button>
      </div>
      <p v-if="logError" class="mt-2 text-sm text-red-400">{{ logError }}</p>
    </div>
  </section>
</template>
