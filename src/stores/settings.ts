import { defineStore } from "pinia";
import { ref, shallowRef } from "vue";
import { api, events, type PendingAccept, type Settings } from "../api";

export const useSettingsStore = defineStore("settings", () => {
  const settings = shallowRef<Settings | null>(null);
  const pendingAccept = shallowRef<PendingAccept | null>(null);
  const saveError = ref<string | null>(null);
  let started = false;

  async function start() {
    if (started) return;
    started = true;
    let gotEvent = false;
    await events.onAutoAccept((p) => {
      gotEvent = true;
      pendingAccept.value = p;
    });
    const [loaded, pending] = await Promise.all([api.settings(), api.autoAcceptState()]);
    settings.value = loaded;
    if (!gotEvent) pendingAccept.value = pending;
  }

  async function update(patch: Partial<Settings>) {
    if (!settings.value) return;
    saveError.value = null;
    try {
      settings.value = await api.saveSettings({ ...settings.value, ...patch });
    } catch (err) {
      saveError.value = String(err);
    }
  }

  function cancelAutoAccept() {
    api.cancelAutoAccept().catch((err) => console.warn("Failed to cancel auto accept", err));
  }

  return { settings, pendingAccept, saveError, start, update, cancelAutoAccept };
});
