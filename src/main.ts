import { getCurrentWindow } from "@tauri-apps/api/window";
import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import OverlayApp from "./overlay/OverlayApp.vue";
import { router } from "./router";
import "./style.css";
import { installLogForwarding } from "./utils/log-forwarding";

installLogForwarding();

// The champ-select overlays load the same page; their window label says which one.
const label = getCurrentWindow().label;
if (label.startsWith("overlay-")) {
  document.documentElement.classList.add("overlay");
  const side = label === "overlay-allies" ? "allies" : "enemies";
  createApp(OverlayApp, { side }).use(createPinia()).mount("#app");
} else {
  createApp(App).use(createPinia()).use(router).mount("#app");
}
