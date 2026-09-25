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
  teamId: number;
  visionScore: number;
  /** Team-relative figures; only SGP lists every participant, so LCU pages have none. */
  metrics: GameMetrics | null;
  participants: { puuid: string; teamId: number }[];
}

/** Shares are fractions of the team total (0.25 = 25%). */
export interface GameMetrics {
  teamSize: number;
  damageShare: number;
  damageTakenShare: number;
  goldShare: number;
  csShare: number;
  visionShare: number;
  killShare: number;
  killParticipation: number;
  healRatio: number;
  soloKills: number;
  enemyMissingPings: number;
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

export type RosterStage = "lobby" | "champSelect" | "inGame";

export interface RosterPlayer {
  puuid: string;
  /** Locked or hovered champion; 0 when none yet. */
  championId: number;
  /** TOP / JUNGLE / MIDDLE / BOTTOM / UTILITY; empty outside role queues. */
  position: string;
  isSelf: boolean;
}

export interface Roster {
  stage: RosterStage;
  gameId: number;
  /** 0 when unknown (champ select does not say). */
  queueId: number;
  allies: RosterPlayer[];
  enemies: RosterPlayer[];
  /** Opponents the client does not identify; during champ select that is all of them. */
  hiddenEnemies: number;
}

export interface PlayerLine {
  puuid: string;
  gameName: string;
  tagLine: string;
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
  damageTaken: number;
  visionScore: number;
  position: string;
}

export interface GameDetail {
  gameId: number;
  queueId: number;
  gameMode: string;
  createdAt: number;
  duration: number;
  source: DataSource;
  /** Blue side (100) first. */
  teams: { teamId: number; win: boolean; kills: number; gold: number; players: PlayerLine[] }[];
}

export type TagTone = "positive" | "negative" | "warning" | "neutral";

export interface PlayerTag {
  id: string;
  label: string;
  tone: TagTone;
  /** Evidence and sample size, shown on hover. */
  detail: string;
  lowConfidence: boolean;
  /** Higher shows first. */
  priority: number;
}

export type SampleScope = "ranked" | "sameQueue" | "all";

export interface PlayerProfile {
  puuid: string;
  source: DataSource;
  scope: SampleScope;
  sampleGames: number;
  wins: number;
  winRate: number;
  avgKills: number;
  avgDeaths: number;
  avgAssists: number;
  avgKda: number;
  team: {
    games: number;
    damageShare: number;
    damageTakenShare: number;
    goldShare: number;
    killParticipation: number;
    csPerMinute: number;
    visionPerMinute: number;
    damagePerGold: number;
  } | null;
  akariScore: { total: number; max: number; games: number; outstanding: boolean; extraordinary: boolean } | null;
  recent: GameResult[];
  topChampions: { championId: number; games: number; wins: number }[];
  position: string;
  tags: PlayerTag[];
}

export interface PlayerPower {
  /** 0–100, 50 = average. */
  power: number;
  tier: "top" | "bottom" | null;
  ally: boolean;
  /** How the index was built. */
  breakdown: string;
}

export interface Advice {
  tone: TagTone;
  /** Player the advice is about. */
  puuid: string;
  title: string;
  detail: string;
}

export interface RosterInsights {
  premades: { name: string; members: string[]; allies: boolean; sharedGames: number }[];
  /** Roster-wide tags (premade, met, gank, lane, horses) per puuid. */
  tags: Record<string, PlayerTag[]>;
  powers: Record<string, PlayerPower>;
  lanes: { position: string; ally: string; enemy: string; allyPower: number; enemyPower: number }[];
  advice: Advice[];
}

export interface Settings {
  autoAccept: boolean;
  /** 0–10 seconds before accepting, leaving time to decline by hand. */
  autoAcceptDelaySecs: number;
  /** Bring the window forward when champ select starts and when the game loads. */
  autoShowPanel: boolean;
}

export interface PendingAccept {
  /** Unix milliseconds at which the match will be accepted. */
  acceptAt: number;
}

/** Page size the backend prefetches for every player in the current game. */
export const PANEL_HISTORY_COUNT = 20;

export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),
  lcuSnapshot: () => invoke<LcuSnapshot>("lcu_snapshot"),
  relaunchAsAdmin: () => invoke<void>("relaunch_as_admin"),
  lookupSummoner: (riotId: string) => invoke<Summoner>("lookup_summoner", { riotId }),
  summonerByPuuid: (puuid: string) => invoke<Summoner>("summoner_by_puuid", { puuid }),
  matchHistory: (puuid: string, start: number, count: number) =>
    invoke<MatchHistoryPage>("match_history", { puuid, start, count }),
  gameData: () => invoke<GameData>("game_data"),
  gameDetail: (gameId: number) => invoke<GameDetail>("game_detail", { gameId }),
  ongoingRoster: () => invoke<Roster | null>("ongoing_roster"),
  playerProfile: (puuid: string, championId: number, queueId: number, position: string) =>
    invoke<PlayerProfile>("player_profile", { puuid, championId, queueId, position }),
  rosterInsights: () => invoke<RosterInsights>("roster_insights"),
  settings: () => invoke<Settings>("settings"),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),
  autoAcceptState: () => invoke<PendingAccept | null>("auto_accept_state"),
  cancelAutoAccept: () => invoke<void>("cancel_auto_accept"),
  logDir: () => invoke<string>("log_dir"),
  openLogDir: () => invoke<void>("open_log_dir"),
  logFrontend: (level: "info" | "warn" | "error", message: string) =>
    invoke<void>("log_frontend", { level, message }),
};

/** LCU game-data images, proxied by the backend's `lcu-asset` protocol. */
export function assetUrl(lcuPath: string): string {
  return `http://lcu-asset.localhost${lcuPath}`;
}

export const events = {
  onLcuSnapshot: (cb: (s: LcuSnapshot) => void): Promise<UnlistenFn> =>
    listen<LcuSnapshot>("lcu://snapshot", (e) => cb(e.payload)),
  onRoster: (cb: (r: Roster | null) => void): Promise<UnlistenFn> =>
    listen<Roster | null>("ongoing://roster", (e) => cb(e.payload)),
  onAutoAccept: (cb: (p: PendingAccept | null) => void): Promise<UnlistenFn> =>
    listen<PendingAccept | null>("auto-accept://state", (e) => cb(e.payload)),
  onGameflowPhase: (cb: (c: PhaseChange) => void): Promise<UnlistenFn> =>
    listen<PhaseChange>("lcu://gameflow-phase", (e) => cb(e.payload)),
};
