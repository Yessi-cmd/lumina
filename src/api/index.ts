import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface AppInfo {
  name: string;
  version: string;
}

export type ConnectionStatus = "disconnected" | "connecting" | "connected";

export interface ClientInfo {
  pid: number;
  port: number;
  platformId: string | null;
  source: "commandLine" | "lockfile";
  /** SGP server name, e.g. 艾欧尼亚; null means match history comes from LCU only. */
  sgpServer: string | null;
}

export interface Summoner {
  puuid: string;
  summonerId: number;
  gameName: string;
  tagLine: string;
  displayName: string;
  summonerLevel: number;
  profileIconId: number;
}

/** `/lol-gameflow/v1/gameflow-phase`, e.g. `None`, `Lobby`, `ChampSelect`, `InProgress`. */
export type GameflowPhase = string;

export interface LcuSnapshot {
  status: ConnectionStatus;
  client: ClientInfo | null;
  summoner: Summoner | null;
  gameflowPhase: GameflowPhase;
  lastError: string | null;
  /** A client is running but only an elevated Lumina can read its credentials. */
  needsAdmin: boolean;
}

export interface PhaseChange {
  phase: GameflowPhase;
  previous: GameflowPhase;
}

export type DataSource = "sgp" | "lcu";
export type GameResult = "win" | "loss" | "remake" | "abort";

/** One game from the queried player's point of view. */
export interface GameSummary {
  gameId: number;
  queueId: number;
  gameMode: string;
  /** Unix milliseconds. */
  createdAt: number;
  /** Seconds. */
  duration: number;
  result: GameResult;
  championId: number;
  champLevel: number;
  kills: number;
  deaths: number;
  assists: number;
  spells: [number, number];
  items: number[];
  cs: number;
  gold: number;
  damageToChampions: number;
  position: string;
}

export interface MatchHistoryPage {
  puuid: string;
  start: number;
  count: number;
  source: DataSource;
  /** Why SGP was not used when the page came from LCU. */
  sgpError: string | null;
  games: GameSummary[];
}

/** Icon values are LCU asset paths; pass them through `assetUrl`. */
export interface GameData {
  champions: Record<string, { name: string; icon: string }>;
  itemIcons: Record<string, string>;
  spellIcons: Record<string, string>;
  queueNames: Record<string, string>;
}

export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),
  lcuSnapshot: () => invoke<LcuSnapshot>("lcu_snapshot"),
  relaunchAsAdmin: () => invoke<void>("relaunch_as_admin"),
  lookupSummoner: (riotId: string) => invoke<Summoner>("lookup_summoner", { riotId }),
  summonerByPuuid: (puuid: string) => invoke<Summoner>("summoner_by_puuid", { puuid }),
  matchHistory: (puuid: string, start: number, count: number) =>
    invoke<MatchHistoryPage>("match_history", { puuid, start, count }),
  gameData: () => invoke<GameData>("game_data"),
};

/** LCU game-data images, proxied by the backend's `lcu-asset` protocol. */
export function assetUrl(lcuPath: string): string {
  return `http://lcu-asset.localhost${lcuPath}`;
}

export const events = {
  onLcuSnapshot: (cb: (s: LcuSnapshot) => void): Promise<UnlistenFn> =>
    listen<LcuSnapshot>("lcu://snapshot", (e) => cb(e.payload)),
  onGameflowPhase: (cb: (c: PhaseChange) => void): Promise<UnlistenFn> =>
    listen<PhaseChange>("lcu://gameflow-phase", (e) => cb(e.payload)),
};
