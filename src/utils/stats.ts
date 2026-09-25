import type { GameResult, GameSummary } from "../api";

export interface PlayerStats {
  /** Games that count towards win rate (remakes and aborted games excluded). */
  counted: number;
  wins: number;
  winRate: number;
  avgKills: number;
  avgDeaths: number;
  avgAssists: number;
  /** Most recent first. */
  recent: GameResult[];
  topChampions: { championId: number; games: number; wins: number }[];
}

export function summarize(games: GameSummary[], recentCount = 10): PlayerStats {
  const counted = games.filter((g) => g.result === "win" || g.result === "loss");
  const wins = counted.filter((g) => g.result === "win").length;
  const n = counted.length || 1;
  const sum = (f: (g: GameSummary) => number) => counted.reduce((acc, g) => acc + f(g), 0);

  const byChampion = new Map<number, { championId: number; games: number; wins: number }>();
  for (const g of counted) {
    const entry = byChampion.get(g.championId) ?? { championId: g.championId, games: 0, wins: 0 };
    entry.games += 1;
    if (g.result === "win") entry.wins += 1;
    byChampion.set(g.championId, entry);
  }

  return {
    counted: counted.length,
    wins,
    winRate: counted.length ? wins / counted.length : 0,
    avgKills: sum((g) => g.kills) / n,
    avgDeaths: sum((g) => g.deaths) / n,
    avgAssists: sum((g) => g.assists) / n,
    recent: games.slice(0, recentCount).map((g) => g.result),
    topChampions: [...byChampion.values()].sort((a, b) => b.games - a.games).slice(0, 3),
  };
}
