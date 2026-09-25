import { defineStore } from "pinia";
import { ref } from "vue";
import { api, type AppInfo } from "../api";

export const useAppStore = defineStore("app", () => {
  const info = ref<AppInfo | null>(null);

  async function load() {
    info.value = await api.appInfo();
  }

  return { info, load };
});
