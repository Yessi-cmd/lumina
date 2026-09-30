<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { api } from "../api";
import AppIcon from "../components/common/AppIcon.vue";
import ToggleSwitch from "../components/common/ToggleSwitch.vue";
import { useAppStore } from "../stores/app";
import { useSettingsStore } from "../stores/settings";

const app = useAppStore();
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

const pushTesting = ref(false);
const pushResult = ref<{ ok: boolean; text: string } | null>(null);

async function testPush() {
  pushTesting.value = true;
  pushResult.value = null;
  try {
    await api.pushTest();
    pushResult.value = { ok: true, text: "已发送，看看手机有没有收到。" };
  } catch (err) {
    pushResult.value = { ok: false, text: String(err) };
  } finally {
    pushTesting.value = false;
  }
}

const PUSH_HINTS: Record<string, string> = {
  bark: "在 Bark App 首页复制设备 Key，也可以填自建服务器的完整地址（如 https://bark.example.com/密钥）。",
  serverchan: "填 Server酱 Turbo 的 SendKey（sct.ftqq.com），消息会转发到微信。",
};

// The slider label follows the thumb while dragging; the value is saved on release.
const delayDraft = ref(0);
watch(
  () => s.value?.autoAcceptDelaySecs,
  (secs) => (delayDraft.value = secs ?? 0),
  { immediate: true },
);
function onDelayCommit() {
  store.update({ autoAcceptDelaySecs: delayDraft.value });
}
</script>

<template>
  <section class="stagger flex max-w-3xl flex-col gap-5">
    <header class="page-header">
      <div class="eyebrow">Settings</div>
      <h1 class="page-title mt-1">设置</h1>
    </header>

    <p v-if="!s" class="text-sm text-zinc-400">加载中…</p>

    <template v-else>
      <div>
        <h2 class="eyebrow mb-2 px-1">对局</h2>
        <div class="card divide-y divide-white/[0.05] overflow-hidden">
          <div class="flex items-center justify-between gap-6 p-4 transition-colors duration-200 hover:bg-white/[0.02]">
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
              v-model.number="delayDraft"
              :style="{ '--fill': `${delayDraft * 10}%` }"
              @change="onDelayCommit"
            />
            <span
              class="w-14 rounded-md bg-amber-400/10 py-0.5 text-center text-sm font-medium text-amber-200 tabular-nums ring-1 ring-amber-400/20 ring-inset"
            >
              {{ delayDraft }} 秒
            </span>
          </div>

          <div class="flex items-center justify-between gap-6 p-4 transition-colors duration-200 hover:bg-white/[0.02]">
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

          <div class="flex items-center justify-between gap-6 p-4 transition-colors duration-200 hover:bg-white/[0.02]">
            <div>
              <div class="font-medium text-zinc-100">选人阶段悬浮窗</div>
              <div class="mt-0.5 text-sm text-zinc-400">
                英雄选择时贴在客户端两侧：左边是队友近期单双排的常用英雄，右边是敌方已选英雄的分路推断和 counter 建议。开启后选人阶段不再弹出主窗口，免得挡住客户端。
              </div>
            </div>
            <ToggleSwitch
              :model-value="s.champSelectOverlay"
              @update:model-value="store.update({ champSelectOverlay: $event })"
            />
          </div>
        </div>
      </div>

      <div>
        <h2 class="eyebrow mb-2 px-1">提醒与赛后</h2>
        <div class="card divide-y divide-white/[0.05] overflow-hidden">
          <div class="flex items-center justify-between gap-6 p-4 transition-colors duration-200 hover:bg-white/[0.02]">
            <div>
              <div class="font-medium text-zinc-100">连败止损提醒</div>
              <div class="mt-0.5 text-sm text-zinc-400">
                排位连败达到这个数、且最近一局在 3 小时内，就在顶部提醒一次；开始排队时若仍在连败中会再提醒。只统计 Lumina 记录到的排位对局。
              </div>
            </div>
            <select
              class="field w-36 shrink-0"
              :value="s.tiltStreak"
              @change="store.update({ tiltStreak: Number(($event.target as HTMLSelectElement).value) })"
            >
              <option :value="0">关闭</option>
              <option :value="2">连败 2 把</option>
              <option :value="3">连败 3 把</option>
              <option :value="4">连败 4 把</option>
              <option :value="5">连败 5 把</option>
            </select>
          </div>

          <div class="flex items-center justify-between gap-6 p-4 transition-colors duration-200 hover:bg-white/[0.02]">
            <div>
              <div class="font-medium text-zinc-100">自动荣誉点赞</div>
              <div class="mt-0.5 text-sm text-zinc-400">
                对局结束出现点赞界面时，自动给一名随机队友点赞。想认真投票的话别开。
              </div>
            </div>
            <ToggleSwitch :model-value="s.autoHonor" @update:model-value="store.update({ autoHonor: $event })" />
          </div>

          <div class="flex items-center justify-between gap-6 p-4" :class="!s.autoHonor && 'opacity-50'">
            <span class="text-sm text-zinc-300">点赞类型</span>
            <select
              class="field w-36 shrink-0"
              :disabled="!s.autoHonor"
              :value="s.honorCategory"
              @change="store.update({ honorCategory: ($event.target as HTMLSelectElement).value })"
            >
              <option value="HEART">友善</option>
              <option value="SHOTCALLER">指挥</option>
              <option value="COOL">酷</option>
            </select>
          </div>
        </div>
      </div>

      <div>
        <h2 class="eyebrow mb-2 px-1">手机推送</h2>
        <div class="card divide-y divide-white/[0.05] overflow-hidden">
          <div class="flex items-center justify-between gap-6 p-4">
            <div>
              <div class="font-medium text-zinc-100">推送渠道</div>
              <div class="mt-0.5 text-sm text-zinc-400">
                排队后离开电脑时，找到对局、进入选人、连败提醒会发到手机。消息由这台电脑直接发给所选渠道。
              </div>
            </div>
            <select
              class="field w-36 shrink-0"
              :value="s.pushProvider"
              @change="store.update({ pushProvider: ($event.target as HTMLSelectElement).value })"
            >
              <option value="off">关闭</option>
              <option value="bark">Bark（iOS）</option>
              <option value="serverchan">Server酱（微信）</option>
            </select>
          </div>

          <template v-if="s.pushProvider !== 'off'">
            <div class="flex items-center gap-4 p-4">
              <span class="w-20 shrink-0 text-sm text-zinc-300">密钥</span>
              <input
                type="password"
                autocomplete="off"
                spellcheck="false"
                class="field min-w-0 flex-1"
                placeholder="粘贴后按回车或点别处保存"
                :value="s.pushKey"
                @change="store.update({ pushKey: ($event.target as HTMLInputElement).value })"
              />
            </div>
            <p class="px-4 py-3 text-xs text-zinc-500">{{ PUSH_HINTS[s.pushProvider] }}</p>

            <div class="flex items-center justify-between gap-6 p-4">
              <span class="text-sm text-zinc-300">找到对局时推送</span>
              <ToggleSwitch
                :model-value="s.pushMatchFound"
                @update:model-value="store.update({ pushMatchFound: $event })"
              />
            </div>
            <div class="flex items-center justify-between gap-6 p-4">
              <span class="text-sm text-zinc-300">进入英雄选择时推送</span>
              <ToggleSwitch
                :model-value="s.pushChampSelect"
                @update:model-value="store.update({ pushChampSelect: $event })"
              />
            </div>
            <div class="flex items-center justify-between gap-6 p-4">
              <span class="text-sm text-zinc-300">连败提醒时推送</span>
              <ToggleSwitch :model-value="s.pushTilt" @update:model-value="store.update({ pushTilt: $event })" />
            </div>

            <div class="flex items-center gap-4 p-4">
              <button class="btn btn-secondary" :disabled="pushTesting || !s.pushKey" @click="testPush">
                {{ pushTesting ? "发送中…" : "发送测试消息" }}
              </button>
              <span
                v-if="pushResult"
                class="min-w-0 text-sm break-all"
                :class="pushResult.ok ? 'text-emerald-400' : 'text-red-400'"
              >
                {{ pushResult.text }}
              </span>
            </div>
          </template>
        </div>
      </div>

      <div>
        <h2 class="eyebrow mb-2 px-1">英雄数据</h2>
        <div class="card divide-y divide-white/[0.05] overflow-hidden">
          <div class="flex items-center justify-between gap-6 p-4 transition-colors duration-200 hover:bg-white/[0.02]">
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

          <div class="flex items-center justify-between gap-6 p-4 transition-colors duration-200 hover:bg-white/[0.02]">
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

      <div>
        <h2 class="eyebrow mb-2 px-1">版本</h2>
        <div class="card flex items-center justify-between gap-6 p-4">
          <div class="min-w-0">
            <div class="font-medium text-zinc-100">Lumina v{{ app.info?.version ?? "…" }}</div>
            <div class="mt-0.5 text-sm text-zinc-400">
              <template v-if="app.checking">正在检查更新…</template>
              <template v-else-if="app.installError">更新失败：{{ app.installError }}</template>
              <template v-else-if="app.updateError">检查更新失败：{{ app.updateError }}</template>
              <template v-else-if="app.update?.available">
                GitHub 上有新版本 v{{ app.update.latest }}。
              </template>
              <template v-else-if="app.update">已是最新版本。</template>
              <template v-else>启动时和之后每 6 小时自动检查 GitHub 上的新版本。</template>
            </div>
          </div>
          <div class="flex shrink-0 gap-2">
            <button
              v-if="app.update?.installable"
              class="btn btn-primary"
              :disabled="app.installProgress !== null"
              @click="app.installUpdate()"
            >
              {{
                app.installProgress === null ? "立即更新" : `下载中 ${Math.round(app.installProgress * 100)}%`
              }}
            </button>
            <button v-else-if="app.update?.available" class="btn btn-primary" @click="app.openRelease()">
              前往下载
            </button>
            <button class="btn btn-secondary" :disabled="app.checking" @click="app.checkUpdate()">
              检查更新
            </button>
          </div>
        </div>
      </div>
    </template>
  </section>
</template>
