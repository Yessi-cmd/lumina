import { createRouter, createWebHashHistory } from "vue-router";

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", name: "home", component: () => import("./views/HomeView.vue") },
    {
      path: "/match-history",
      name: "match-history",
      component: () => import("./views/MatchHistoryView.vue"),
    },
    { path: "/rank", name: "rank", component: () => import("./views/RankView.vue") },
    {
      path: "/ongoing-game",
      name: "ongoing-game",
      component: () => import("./views/OngoingGameView.vue"),
    },
    { path: "/settings", name: "settings", component: () => import("./views/SettingsView.vue") },
  ],
});
