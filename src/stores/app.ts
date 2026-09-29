import { defineStore } from "pinia";
import { ref } from "vue";
import { api, events, type AppInfo, type UpdateInfo } from "../api";

/** Re-check while the app stays open. */
const UPDATE_INTERVAL_MS = 6 * 60 * 60 * 1000;
const DISMISSED_KEY = "lumina.dismissedUpdate";

export const useAppStore = defineStore("app", () => {
  const info = ref<AppInfo | null>(null);
  const update = ref<UpdateInfo | null>(null);
  const updateError = ref<string | null>(null);
  const checking = ref(false);
  /** Version whose banner the user closed; a later release shows again. */
  const dismissed = ref(readDismissed());
  let timer: ReturnType<typeof setInterval> | undefined;

  async function load() {
    info.value = await api.appInfo();
  }

  async function checkUpdate() {
    checking.value = true;
    try {
      update.value = await api.checkUpdate();
      updateError.value = null;
    } catch (err) {
      updateError.value = String(err);
      console.warn("Update check failed", err);
    } finally {
      checking.value = false;
    }
  }

  function watchUpdates() {
    if (timer) return;
    checkUpdate();
    timer = setInterval(checkUpdate, UPDATE_INTERVAL_MS);
  }

  function dismissUpdate() {
    const latest = update.value?.latest;
    if (!latest) return;
    dismissed.value = latest;
    try {
      localStorage.setItem(DISMISSED_KEY, latest);
    } catch {
      // Storage unavailable: the banner just comes back next launch.
    }
  }

  /** Download progress 0–1 while installing; null otherwise. */
  const installProgress = ref<number | null>(null);
  const installError = ref<string | null>(null);

  /** Downloads and starts the installer; the app quits once it runs. */
  async function installUpdate() {
    if (installProgress.value !== null) return;
    installProgress.value = 0;
    installError.value = null;
    const unlisten = await events.onUpdateProgress((p) => {
      installProgress.value = p.total > 0 ? p.downloaded / p.total : 0;
    });
    try {
      await api.installUpdate();
    } catch (err) {
      installError.value = String(err);
      installProgress.value = null;
    } finally {
      unlisten();
    }
  }

  function openRelease() {
    const url = update.value?.url;
    if (url) api.openRelease(url).catch((err) => console.error("Failed to open release", err));
  }

  return {
    info,
    load,
    update,
    updateError,
    checking,
    dismissed,
    checkUpdate,
    watchUpdates,
    dismissUpdate,
    openRelease,
    installProgress,
    installError,
    installUpdate,
  };
});

function readDismissed(): string | null {
  try {
    return localStorage.getItem(DISMISSED_KEY);
  } catch {
    return null;
  }
}
