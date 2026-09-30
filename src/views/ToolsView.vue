<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { api, assetUrl, type Skin } from "../api";
import ChampionPicker from "../components/champion/ChampionPicker.vue";
import { useGameDataStore } from "../stores/gameData";
import { useLcuStore } from "../stores/lcu";

const lcu = useLcuStore();
const gd = useGameDataStore();
const connected = computed(() => lcu.connected);

/** Result line under a tool: green when it worked, red with the reason when not. */
interface Note {
  ok: boolean;
  text: string;
}
function noteOf(err: unknown): Note {
  return { ok: false, text: String(err) };
}

/** Success notes fade after a few seconds; errors stay until the next action. */
const NOTE_LIFETIME_MS = 4000;
function flash(target: { value: Note | null }, text: string) {
  const note: Note = { ok: true, text };
  target.value = note;
  setTimeout(() => {
    if (target.value === note) target.value = null;
  }, NOTE_LIFETIME_MS);
}

// --- Chat status ---------------------------------------------------------------------
const AVAILABILITY = [
  { id: "chat", label: "在线" },
  { id: "away", label: "离开" },
  { id: "offline", label: "隐身" },
];
const availability = ref("");
const statusMessage = ref("");
const statusNote = ref<Note | null>(null);
const savingStatus = ref(false);

const availabilityText = computed(() => {
  const known = AVAILABILITY.find((a) => a.id === availability.value);
  if (known) return known.label;
  if (availability.value === "dnd") return "游戏中";
  if (availability.value === "mobile") return "手机在线";
  return "未知";
});

async function loadStatus() {
  try {
    const status = await api.chatStatus();
    availability.value = status.availability;
    statusMessage.value = status.statusMessage;
  } catch (err) {
    statusNote.value = noteOf(err);
  }
}

async function setAvailability(id: string) {
  statusNote.value = null;
  try {
    await api.setChatAvailability(id);
    availability.value = id;
    flash(statusNote, "已切换。");
  } catch (err) {
    statusNote.value = noteOf(err);
  }
}

async function saveStatusMessage() {
  savingStatus.value = true;
  statusNote.value = null;
  try {
    await api.setChatStatusMessage(statusMessage.value);
    flash(statusNote, "签名已保存。");
  } catch (err) {
    statusNote.value = noteOf(err);
  } finally {
    savingStatus.value = false;
  }
}

// --- Career background ---------------------------------------------------------------
const picking = ref(false);
const championId = ref<number | null>(null);
const skins = ref<Skin[]>([]);
const loadingSkins = ref(false);
const backgroundId = ref(0);
const backgroundNote = ref<Note | null>(null);
let skinRequest = 0;

async function loadBackground() {
  try {
    backgroundId.value = await api.profileBackground();
  } catch (err) {
    console.warn("Failed to read the career background", err);
  }
}

async function chooseChampion(id: number) {
  picking.value = false;
  championId.value = id;
  skins.value = [];
  backgroundNote.value = null;
  const request = ++skinRequest;
  loadingSkins.value = true;
  try {
    const found = await api.championSkins(id);
    if (request === skinRequest) skins.value = found;
  } catch (err) {
    if (request === skinRequest) backgroundNote.value = noteOf(err);
  } finally {
    if (request === skinRequest) loadingSkins.value = false;
  }
}

async function setBackground(skin: Skin) {
  backgroundNote.value = null;
  try {
    await api.setProfileBackground(skin.id);
    backgroundId.value = skin.id;
    flash(backgroundNote, `生涯背景已设为「${skin.name}」。`);
  } catch (err) {
    backgroundNote.value = noteOf(err);
  }
}

// --- Restart -------------------------------------------------------------------------
const confirmingRestart = ref(false);
const restartNote = ref<Note | null>(null);
let confirmTimer: ReturnType<typeof setTimeout> | undefined;

async function restart() {
  if (!confirmingRestart.value) {
    confirmingRestart.value = true;
    clearTimeout(confirmTimer);
    confirmTimer = setTimeout(() => (confirmingRestart.value = false), 4000);
    return;
  }
  clearTimeout(confirmTimer);
  confirmingRestart.value = false;
  restartNote.value = null;
  try {
    await api.restartClient();
    restartNote.value = { ok: true, text: "已发送重启请求，客户端窗口会重新打开，Lumina 会自动重连。" };
  } catch (err) {
    restartNote.value = noteOf(err);
  }
}

function load() {
  if (!lcu.connected) return;
  loadStatus();
  loadBackground();
}
onMounted(load);
watch(connected, load);
</script>

<template>
  <section class="stagger flex max-w-4xl flex-col gap-5">
    <header class="page-header">
      <div class="eyebrow">Tools</div>
      <h1 class="page-title mt-1">工具</h1>
    </header>

    <p v-if="!connected" class="card p-4 text-sm text-zinc-400">这些工具直接操作客户端，需要先连接英雄联盟客户端。</p>

    <template v-else>
      <div class="card p-5">
        <div class="flex items-start justify-between gap-6">
          <div>
            <div class="font-medium text-zinc-100">在线状态与签名</div>
            <div class="mt-0.5 text-sm text-zinc-400">
              当前：{{ availabilityText }}。隐身后好友看到你离线，但你仍可以正常排队。
            </div>
          </div>
          <div class="segmented shrink-0">
            <button
              v-for="a in AVAILABILITY"
              :key="a.id"
              class="segment"
              :class="availability === a.id && 'segment-active'"
              @click="setAvailability(a.id)"
            >
              {{ a.label }}
            </button>
          </div>
        </div>
        <div class="mt-4 flex items-center gap-3">
          <input
            v-model="statusMessage"
            class="field min-w-0 flex-1"
            maxlength="100"
            placeholder="状态签名（好友列表里显示）"
            @keydown.enter="saveStatusMessage"
          />
          <button class="btn btn-secondary shrink-0" :disabled="savingStatus" @click="saveStatusMessage">
            保存签名
          </button>
        </div>
        <p v-if="statusNote" class="mt-2 text-sm break-all" :class="statusNote.ok ? 'text-emerald-400' : 'text-red-400'">
          {{ statusNote.text }}
        </p>
      </div>

      <div class="card p-5">
        <div class="font-medium text-zinc-100">生涯背景</div>
        <div class="mt-0.5 text-sm text-zinc-400">
          选一个英雄，再点它的皮肤，就把个人资料的背景换成这套皮肤。未拥有的皮肤客户端可能不接受，会显示原因。
        </div>
        <div class="mt-3 flex items-center gap-3">
          <button class="btn btn-secondary" @click="picking = !picking">
            {{ championId ? "换一个英雄" : "选择英雄" }}
          </button>
          <div v-if="championId" class="flex items-center gap-2">
            <img :src="gd.championIcon(championId)" class="size-8 rounded-md bg-zinc-800" />
            <span class="text-sm text-zinc-200">{{ gd.championName(championId) }}</span>
          </div>
        </div>
        <ChampionPicker v-if="picking" class="mt-3" @pick="chooseChampion" @close="picking = false" />

        <p v-if="loadingSkins" class="mt-3 text-sm text-zinc-500">正在读取皮肤…</p>
        <div v-else-if="skins.length" class="mt-3 grid grid-cols-[repeat(auto-fill,minmax(120px,1fr))] gap-2">
          <button
            v-for="skin in skins"
            :key="skin.id"
            class="group relative overflow-hidden rounded-lg border text-left transition-colors duration-200"
            :class="
              skin.id === backgroundId
                ? 'border-amber-400/70 ring-1 ring-amber-400/40'
                : 'border-white/[0.08] hover:border-white/25'
            "
            @click="setBackground(skin)"
          >
            <img
              v-if="skin.tilePath"
              :src="assetUrl(skin.tilePath)"
              class="aspect-square w-full bg-zinc-800 object-cover transition-transform duration-500 group-hover:scale-105"
              loading="lazy"
            />
            <div v-else class="flex aspect-square w-full items-center justify-center bg-zinc-800 text-xs text-zinc-500">
              无图
            </div>
            <div class="bg-zinc-950/80 px-2 py-1.5">
              <div class="truncate text-xs text-zinc-100">{{ skin.name }}</div>
              <div class="text-[10px]" :class="skin.owned ? 'text-emerald-400' : 'text-zinc-500'">
                {{ skin.id === backgroundId ? "当前背景" : skin.owned ? "已拥有" : "未拥有" }}
              </div>
            </div>
          </button>
        </div>
        <p
          v-if="backgroundNote"
          class="mt-3 text-sm break-all"
          :class="backgroundNote.ok ? 'text-emerald-400' : 'text-red-400'"
        >
          {{ backgroundNote.text }}
        </p>
      </div>

      <div class="card flex items-start justify-between gap-6 p-5">
        <div>
          <div class="font-medium text-zinc-100">重启客户端窗口</div>
          <div class="mt-0.5 text-sm text-zinc-400">
            客户端界面卡住或白屏时使用。只重启客户端窗口，正在进行的游戏不受影响；排队和选人会中断。
          </div>
          <p v-if="restartNote" class="mt-2 text-sm break-all" :class="restartNote.ok ? 'text-emerald-400' : 'text-red-400'">
            {{ restartNote.text }}
          </p>
        </div>
        <button class="btn shrink-0" :class="confirmingRestart ? 'btn-primary' : 'btn-secondary'" @click="restart">
          {{ confirmingRestart ? "再点一次确认" : "重启客户端" }}
        </button>
      </div>
    </template>
  </section>
</template>
