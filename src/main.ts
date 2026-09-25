import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import { router } from "./router";
import "./style.css";
import { installLogForwarding } from "./utils/log-forwarding";

installLogForwarding();

createApp(App).use(createPinia()).use(router).mount("#app");
