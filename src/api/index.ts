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

export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),
  lcuSnapshot: () => invoke<LcuSnapshot>("lcu_snapshot"),
  relaunchAsAdmin: () => invoke<void>("relaunch_as_admin"),
};

export const events = {
  onLcuSnapshot: (cb: (s: LcuSnapshot) => void): Promise<UnlistenFn> =>
    listen<LcuSnapshot>("lcu://snapshot", (e) => cb(e.payload)),
  onGameflowPhase: (cb: (c: PhaseChange) => void): Promise<UnlistenFn> =>
    listen<PhaseChange>("lcu://gameflow-phase", (e) => cb(e.payload)),
};
