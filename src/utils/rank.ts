import type { RankPoint } from "../api";

export const TIERS: Record<string, { name: string; color: string }> = {
  IRON: { name: "黑铁", color: "text-zinc-400" },
  BRONZE: { name: "青铜", color: "text-orange-300" },
  SILVER: { name: "白银", color: "text-slate-300" },
  GOLD: { name: "黄金", color: "text-amber-300" },
  PLATINUM: { name: "铂金", color: "text-teal-300" },
  EMERALD: { name: "翡翠", color: "text-emerald-300" },
  DIAMOND: { name: "钻石", color: "text-sky-300" },
  MASTER: { name: "大师", color: "text-fuchsia-300" },
  GRANDMASTER: { name: "宗师", color: "text-red-300" },
  CHALLENGER: { name: "王者", color: "text-yellow-200" },
};

const TIER_ORDER = ["IRON", "BRONZE", "SILVER", "GOLD", "PLATINUM", "EMERALD", "DIAMOND"];
const DIVISIONS = ["IV", "III", "II", "I"];
const APEX_TIERS = ["MASTER", "GRANDMASTER", "CHALLENGER"];
/** Master starts where Diamond I ends; same scale as the backend's `score`. */
const APEX_BASE = 2800;

export function tierText(tier: string, division: string): string {
  if (!tier) return "未定级";
  const name = TIERS[tier]?.name ?? tier;
  const apex = APEX_TIERS.includes(tier);
  return apex || !division || division === "NA" ? name : `${name} ${division}`;
}

/** Rank as one number: every division is 100 LP, Master and above share one scale. */
export function rankScore(p: Pick<RankPoint, "tier" | "division" | "lp">): number | null {
  if (APEX_TIERS.includes(p.tier)) return APEX_BASE + p.lp;
  const tier = TIER_ORDER.indexOf(p.tier);
  const division = DIVISIONS.indexOf(p.division);
  if (tier < 0 || division < 0) return null;
  return tier * 400 + division * 100 + p.lp;
}

/** Axis label of a score, e.g. 2100 → 翡翠 III. */
export function scoreLabel(score: number): string {
  if (score >= APEX_BASE) return `大师 ${score - APEX_BASE} 点`;
  const tier = TIER_ORDER[Math.min(TIER_ORDER.length - 1, Math.floor(score / 400))];
  const division = DIVISIONS[Math.floor((score % 400) / 100)];
  const lp = score % 100;
  return `${TIERS[tier].name} ${division}${lp ? ` ${lp}` : ""}`;
}

export function signed(n: number): string {
  return n > 0 ? `+${n}` : String(n);
}

/** Text colour of a gain or loss. */
export function deltaColor(n: number): string {
  if (n > 0) return "text-emerald-400";
  if (n < 0) return "text-red-400";
  return "text-zinc-400";
}

export interface ChampionLp {
  championId: number;
  games: number;
  wins: number;
  net: number;
}

export interface RankSummary {
  /** LP change since local midnight, and over the last 7 days. */
  today: number;
  week: number;
  /** All recorded changes added up. */
  total: number;
  /** Games with a known result. */
  games: number;
  wins: number;
  losses: number;
  /** Average LP of a win / a loss; null without such games. */
  avgWin: number | null;
  avgLoss: number | null;
  longestWinStreak: number;
  longestLossStreak: number;
  /** The run the record ends with: positive for wins, negative for losses. */
  currentStreak: number;
  champions: ChampionLp[];
}

const DAY = 86_400_000;

/** `points` are one queue's, oldest first. */
export function summarize(points: RankPoint[], now: number): RankSummary {
  const midnight = new Date(now);
  midnight.setHours(0, 0, 0, 0);
  const since = { today: midnight.getTime(), week: now - 7 * DAY };
  const out: RankSummary = {
    today: 0,
    week: 0,
    total: 0,
    games: 0,
    wins: 0,
    losses: 0,
    avgWin: null,
    avgLoss: null,
    longestWinStreak: 0,
    longestLossStreak: 0,
    currentStreak: 0,
    champions: [],
  };
  let winLp = 0;
  let lossLp = 0;
  let run = 0;
  const byChampion = new Map<number, ChampionLp>();

  for (const p of points) {
    if (p.delta !== null) {
      out.total += p.delta;
      if (p.at >= since.today) out.today += p.delta;
      if (p.at >= since.week) out.week += p.delta;
    }
    if (p.outcome === null) continue;
    const win = p.outcome === "win";
    out.games++;
    if (win) {
      out.wins++;
      winLp += p.delta ?? 0;
    } else {
      out.losses++;
      lossLp += p.delta ?? 0;
    }
    // A run continues while results keep the same sign.
    run = win ? (run > 0 ? run + 1 : 1) : run < 0 ? run - 1 : -1;
    out.longestWinStreak = Math.max(out.longestWinStreak, run);
    out.longestLossStreak = Math.max(out.longestLossStreak, -run);

    if (p.championId !== null && p.delta !== null) {
      const entry = byChampion.get(p.championId) ?? {
        championId: p.championId,
        games: 0,
        wins: 0,
        net: 0,
      };
      entry.games++;
      entry.wins += win ? 1 : 0;
      entry.net += p.delta;
      byChampion.set(p.championId, entry);
    }
  }
  out.currentStreak = run;
  if (out.wins) out.avgWin = winLp / out.wins;
  if (out.losses) out.avgLoss = lossLp / out.losses;
  out.champions = [...byChampion.values()].sort((a, b) => b.games - a.games || b.net - a.net);
  return out;
}
