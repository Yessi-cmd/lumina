import React from 'react';
import {AbsoluteFill, Audio, staticFile, useCurrentFrame, useVideoConfig} from 'remotion';

export const AMBER = '#f59e0b';
export const FONT =
  '"Segoe UI Variable Text","Segoe UI","Microsoft YaHei UI","PingFang SC",system-ui,sans-serif';

// ---------- math / easing ----------
const cl = (x: number, a = 0, b = 1) => Math.max(a, Math.min(b, x));
const ease = (t: number) => 1 - Math.pow(1 - t, 4);
const back = (t: number) => {
  const c1 = 1.70158;
  const c3 = c1 + 1;
  return 1 + c3 * Math.pow(t - 1, 3) + c1 * Math.pow(t - 1, 2);
};
const lin = (t: number, s: number, d: number) => cl((t - s) / d);
/** Eased 0..1 progress. */
const pr = (t: number, s: number, d: number) => ease(lin(t, s, d));
/** Same, with overshoot (a "stamp"). */
const pb = (t: number, s: number, d: number) => back(lin(t, s, d));
const rand = (n: number) => {
  const x = Math.sin(n * 127.1 + 311.7) * 43758.5453;
  return x - Math.floor(x);
};
const shake = (t: number, s: number, amp: number, dur = 0.5): [number, number] => {
  const k = lin(t, s, dur);
  if (k <= 0 || k >= 1) return [0, 0];
  const d = (1 - k) * (1 - k) * amp;
  return [Math.sin(t * 90) * d, Math.cos(t * 77) * d];
};

// ---------- timeline (seconds) ----------
const START = [0, 3, 7, 11, 15, 18];
const END = [3.4, 7.4, 11.4, 15.4, 18.4, 22];
const FLASHES: [number, number][] = [
  [0.9, 0.9],
  [3.1, 0.55],
  [7.1, 0.55],
  [11.1, 0.55],
  [15.1, 0.55],
  [18.1, 1],
];

// ---------- effect primitives ----------
export const Ring: React.FC<{age: number; x: number; y: number; size?: number; color?: string; w?: number; life?: number}> = ({
  age,
  x,
  y,
  size = 900,
  color = '#fde68a',
  w = 5,
  life = 1,
}) => {
  if (age < 0 || age > life) return null;
  const k = age / life;
  return (
    <div
      style={{
        position: 'absolute',
        left: x - size / 2,
        top: y - size / 2,
        width: size,
        height: size,
        borderRadius: '50%',
        border: `${w * (1 - k) + 1}px solid ${color}`,
        opacity: (1 - k) * 0.9,
        transform: `scale(${0.05 + ease(k) * 0.95})`,
        boxShadow: `0 0 40px ${color}, inset 0 0 40px ${color}`,
      }}
    />
  );
};

export const Burst: React.FC<{
  age: number;
  x: number;
  y: number;
  n?: number;
  power?: number;
  color?: string;
  seed?: number;
  size?: number;
  life?: number;
}> = ({age, x, y, n = 36, power = 500, color = '#fde68a', seed = 1, size = 8, life = 1.5}) => {
  if (age < 0 || age > life) return null;
  const k = age / life;
  return (
    <>
      {Array.from({length: n}, (_, i) => {
        const a = rand(seed + i * 3) * Math.PI * 2;
        const sp = power * (0.25 + rand(seed + i * 3 + 1));
        const d = (sp * (1 - Math.exp(-3.5 * age))) / 3.5;
        const px = x + Math.cos(a) * d;
        const py = y + Math.sin(a) * d + 160 * age * age;
        const s = size * (0.4 + rand(seed + i * 3 + 2)) * (1 - k);
        return (
          <div
            key={i}
            style={{position: 'absolute', left: px - s / 2, top: py - s / 2, width: s, height: s, borderRadius: '50%', background: color, opacity: 1 - k, boxShadow: `0 0 ${s * 3}px ${color}`}}
          />
        );
      })}
    </>
  );
};

const Chars: React.FC<{
  text: string;
  t: number;
  start: number;
  stagger?: number;
  dur?: number;
  y?: number;
  blur?: number;
  scale?: number;
  style?: React.CSSProperties;
}> = ({text, t, start, stagger = 0.04, dur = 0.6, y = 50, blur = 18, scale = 1.5, style}) => (
  <span style={style}>
    {text.split('').map((ch, i) => {
      const p = pr(t, start + i * stagger, dur);
      return (
        <span
          key={i}
          style={{
            display: 'inline-block',
            whiteSpace: 'pre',
            opacity: p,
            transform: `translateY(${(1 - p) * y}px) scale(${scale - (scale - 1) * p})`,
            filter: p < 1 ? `blur(${(1 - p) * blur}px)` : undefined,
          }}
        >
          {ch}
        </span>
      );
    })}
  </span>
);

const Headline: React.FC<{title: string; parts: [string, boolean][]; t: number}> = ({title, parts, t}) => {
  let idx = 0;
  const tp = pr(t, 0.05, 0.5);
  return (
    <>
      <div style={{position: 'absolute', left: 140, top: 100, fontSize: 30, letterSpacing: 10, color: AMBER, fontWeight: 700, opacity: tp, transform: `translateX(${(1 - tp) * -40}px)`}}>
        {title}
      </div>
      <div style={{position: 'absolute', left: 140, top: 150, fontSize: 80, fontWeight: 800, letterSpacing: 2, whiteSpace: 'nowrap'}}>
        {parts.map(([txt, em], pi) => {
          const start = 0.15 + idx * 0.045;
          idx += txt.length;
          return (
            <Chars key={pi} text={txt} t={t} start={start} stagger={0.045} style={em ? {color: AMBER, textShadow: `0 0 40px ${AMBER}99`} : undefined} />
          );
        })}
      </div>
      <div style={{position: 'absolute', left: 140, top: 262, height: 4, width: pr(t, 0.4, 0.9) * 520, borderRadius: 2, background: `linear-gradient(90deg,${AMBER},transparent)`, boxShadow: `0 0 20px ${AMBER}`}} />
    </>
  );
};

export const Logo: React.FC<{t: number; start: number; band: number; top: number; size: number; from: number; sweep?: boolean}> = ({t, start, band, top, size, from}) => (
  <div style={{position: 'absolute', left: 0, right: 0, top, display: 'flex', justifyContent: 'center', gap: size * 0.12}}>
    {'LUMINA'.split('').map((ch, i) => {
      const p = pb(t, start + i * 0.07, 0.65);
      const e = pr(t, start + i * 0.07, 0.65);
      const h = Math.max(0, 1 - Math.abs(band - i) / 1.3);
      const d = (1 - e) * 22;
      return (
        <span
          key={i}
          style={{
            fontSize: size,
            lineHeight: 1,
            fontWeight: 800,
            display: 'inline-block',
            opacity: Math.min(1, e * 2),
            transform: `scale(${from - (from - 1) * p})`,
            filter: `blur(${(1 - e) * 30}px) drop-shadow(${d}px 0 0 rgba(255,45,85,.65)) drop-shadow(${-d}px 0 0 rgba(34,211,238,.65)) drop-shadow(0 0 ${22 + h * 60}px rgba(245,158,11,${0.45 + h * 0.5}))`,
            color: `color-mix(in srgb,#fff ${h * 100}%,#fbbf24)`,
          }}
        >
          {ch}
        </span>
      );
    })}
  </div>
);

// ---------- backdrop ----------
const FIELD = Array.from({length: 70}, (_, i) => ({x: rand(i) * 1920, y: rand(i + 100) * 1080, z: 0.4 + rand(i + 200) * 1.3, s: 1.5 + rand(i + 300) * 3.5, c: rand(i + 400)}));

export const Backdrop: React.FC<{t: number}> = ({t}) => {
  const grid = 0.18 + 0.4 * Math.max(Math.exp(-Math.pow((t - 1.5) / 1.4, 2)), Math.exp(-Math.pow((t - 19) / 1.2, 2)));
  const red = Math.exp(-Math.pow((t - 9) / 2.2, 2));
  return (
    <AbsoluteFill>
      <div style={{position: 'absolute', width: 900, height: 900, borderRadius: '50%', filter: 'blur(140px)', background: '#f59e0b30', left: -250 + Math.sin(t / 3) * 80, top: -250 + Math.cos(t / 2.5) * 40}} />
      <div style={{position: 'absolute', width: 1000, height: 1000, borderRadius: '50%', filter: 'blur(150px)', background: '#3b82f630', right: -350 + Math.cos(t / 4) * -80, bottom: -350}} />
      <div style={{position: 'absolute', width: 800, height: 800, borderRadius: '50%', filter: 'blur(140px)', background: '#f43f5e', opacity: red * 0.2, right: -100, top: -200}} />
      <div
        style={{
          position: 'absolute',
          left: -700,
          right: -700,
          bottom: -260,
          height: 900,
          transform: 'perspective(900px) rotateX(72deg)',
          transformOrigin: '50% 100%',
          backgroundImage: `linear-gradient(rgba(245,158,11,.5) 2px, transparent 2px), linear-gradient(90deg, rgba(245,158,11,.5) 2px, transparent 2px)`,
          backgroundSize: '120px 120px',
          backgroundPosition: `0 ${(t * 70) % 120}px`,
          maskImage: 'linear-gradient(transparent 5%, black 75%)',
          opacity: grid,
        }}
      />
      {FIELD.map((p, i) => {
        const y = ((((p.y - t * 30 * p.z) % 1180) + 1180) % 1180) - 50;
        const x = p.x + Math.sin(t * 0.6 + i) * 22 * p.z;
        const tw = 0.35 + 0.65 * Math.abs(Math.sin(t * 1.3 + i));
        const color = p.c < 0.6 ? '#fde68a' : p.c < 0.85 ? '#ffffff' : '#7dd3fc';
        const s = p.s * p.z;
        return <div key={i} style={{position: 'absolute', left: x, top: y, width: s, height: s, borderRadius: '50%', background: color, opacity: tw * 0.75, boxShadow: `0 0 ${s * 4}px ${color}`}} />;
      })}
    </AbsoluteFill>
  );
};

const Scene: React.FC<{i: number; t: number; children: React.ReactNode}> = ({i, t, children}) => {
  const a = i === 0 ? -1 : START[i];
  const b = END[i];
  if (t < a || t > b) return null;
  const inP = ease(cl((t - a) / 0.5));
  const outP = i === END.length - 1 ? 0 : ease(cl((t - (b - 0.4)) / 0.4));
  const scale = (0.82 + 0.18 * inP) * (1 + 0.3 * outP);
  const blur = (1 - inP) * 16 + outP * 18;
  return (
    <AbsoluteFill style={{opacity: cl((t - a) / 0.3) * (1 - outP), transform: `scale(${scale})`, filter: blur > 0.2 ? `blur(${blur}px)` : undefined}}>
      {children}
    </AbsoluteFill>
  );
};

// ---------- S1: intro ----------
const Intro: React.FC<{t: number}> = ({t}) => {
  const line = pr(t, 0.15, 0.6);
  const lineFade = 1 - lin(t, 0.85, 0.15);
  const age = t - 0.88;
  const band = -1.5 + 8.5 * lin(t, 1.75, 0.9);
  const glow = pr(t, 0.88, 1.0) * (0.8 + 0.2 * Math.sin(t * 3));
  return (
    <>
      <div style={{position: 'absolute', left: 960 - 900 * line, width: 1800 * line, top: 538, height: 4, borderRadius: 2, background: 'linear-gradient(90deg,transparent,#fff,transparent)', boxShadow: '0 0 30px 6px #fde68a', opacity: lineFade}} />
      <div style={{position: 'absolute', left: 260, top: 140, width: 1400, height: 800, background: 'radial-gradient(closest-side, rgba(245,158,11,.38), transparent)', opacity: glow}} />
      <Ring age={age} x={960} y={540} size={1900} />
      <Ring age={age - 0.12} x={960} y={540} size={1300} color={AMBER} />
      <Burst age={age} x={960} y={540} n={80} power={950} seed={7} size={9} />
      <Logo t={t} start={0.9} band={band} top={330} size={210} from={2.6} />
      <div style={{position: 'absolute', left: 0, right: 0, top: 620, textAlign: 'center', fontSize: 44, letterSpacing: 10, color: '#c1c6d0'}}>
        <Chars text="轻量、高性能的英雄联盟桌面助手" t={t} start={2.0} stagger={0.045} dur={0.5} y={24} blur={10} scale={1} />
      </div>
    </>
  );
};

// ---------- S2: roster scan ----------
type Tone = 'bad' | 'good' | 'neu';
export const TONE: Record<Tone, {bg: string; fg: string}> = {
  bad: {bg: '#3a1d22', fg: '#fb7185'},
  good: {bg: '#3a2c10', fg: '#fbbf24'},
  neu: {bg: '#1f2a3d', fg: '#7dd3fc'},
};
type Player = [string, string, [string, Tone][]];
const BLUE: Player[] = [
  ['夜行者', '白银 II', [['三连败', 'bad'], ['练英雄', 'neu']]],
  ['风语', '黄金 IV', [['绝活 · 亚索', 'good'], ['单杀多', 'good']]],
  ['小熊软糖', '白银 I', [['闪现异位', 'bad']]],
  ['Moonlit', '铂金 III', [['开黑推断', 'neu'], ['连胜 5', 'good']]],
  ['阿杰', '黄金 II', [['近期异常', 'bad'], ['遇见过', 'neu']]],
];
const RED: Player[] = [
  ['Rainfall', '钻石 IV', [['连胜 7', 'good'], ['输出第一', 'good']]],
  ['老王', '黄金 I', [['补刀弱', 'bad']]],
  ['Kiyo', '铂金 II', [['绝活 · 劫', 'good']]],
  ['一只小猫', '白银 III', [['练英雄', 'neu'], ['连败 4', 'bad']]],
  ['Zed丶', '黄金 III', [['闪现异位', 'bad'], ['遇见过', 'neu']]],
];
const BEAM0 = 1.4;
const BEAM_D = 2.0;
const BEAM_X0 = -80;
const BEAM_XW = 1804;

const Roster: React.FC<{l: number}> = ({l}) => {
  const bx = BEAM_X0 + BEAM_XW * lin(l, BEAM0, BEAM_D);
  const beamOn = l > BEAM0 - 0.1 && l < BEAM0 + BEAM_D + 0.3 ? 1 : 0;
  const players = [...BLUE, ...RED];
  let scanned = 0;
  const cards = players.map((p, i) => {
    const col = i % 5;
    const row = i < 5 ? 0 : 1;
    const cx = col * 336 + 150;
    const tc = BEAM0 + (BEAM_D * (cx - BEAM_X0)) / BEAM_XW;
    const sc = l - tc;
    if (sc > 0) scanned++;
    const e = pr(l, 0.35 + (col + row * 0.6) * 0.12, 0.9);
    const flash = sc > 0 ? Math.exp(-sc * 4.5) : 0;
    const team = row ? '#fb7185' : '#60a5fa';
    return (
      <div
        key={p[0]}
        style={{
          position: 'absolute',
          left: col * 336,
          top: row * 270,
          width: 300,
          height: 230,
          padding: 22,
          borderRadius: 22,
          background: 'linear-gradient(145deg, rgba(255,255,255,.09), rgba(255,255,255,.02))',
          border: `1.5px solid color-mix(in srgb, ${AMBER} ${flash * 100}%, rgba(255,255,255,.14))`,
          boxShadow: `0 0 ${flash * 70}px rgba(245,158,11,${flash * 0.85}), 0 30px 60px rgba(0,0,0,.5)`,
          opacity: Math.min(1, e * 2),
          filter: `brightness(${0.7 + 0.3 * lin(sc, 0, 0.2)})`,
          transform: `translate3d(${(1 - e) * 400}px,${(1 - e) * -120}px,${(1 - e) * -1200}px) rotateY(${(1 - e) * 80}deg)`,
        }}
      >
        <div style={{display: 'flex', alignItems: 'center', gap: 16}}>
          <div style={{width: 62, height: 62, borderRadius: '50%', background: `linear-gradient(135deg, ${team}, #1d2129)`, display: 'grid', placeItems: 'center', fontSize: 28, fontWeight: 800, boxShadow: `0 0 20px ${team}66`}}>
            {p[0][0]}
          </div>
          <div>
            <div style={{fontSize: 28, fontWeight: 700}}>{p[0]}</div>
            <div style={{fontSize: 20, color: '#969dab', marginTop: 2}}>{p[1]}</div>
          </div>
        </div>
        <div style={{display: 'flex', flexWrap: 'wrap', gap: 8, marginTop: 22}}>
          {p[2].map(([label, tone], j) => {
            const q = pb(l, tc + 0.1 + 0.12 * j, 0.35);
            const c = TONE[tone];
            return (
              <span key={label} style={{fontSize: 21, padding: '6px 14px', borderRadius: 999, whiteSpace: 'nowrap', background: c.bg, color: c.fg, opacity: Math.min(1, q * 3), transform: `scale(${2.2 - 1.2 * q})`, boxShadow: `0 0 ${(1 - q) * 40}px ${c.fg}`}}>
                {label}
              </span>
            );
          })}
        </div>
      </div>
    );
  });
  return (
    <div style={{position: 'absolute', inset: 0, perspective: 2200, perspectiveOrigin: '50% 40%'}}>
      <div style={{position: 'absolute', left: 138, top: 340, width: 1644, height: 560, transformStyle: 'preserve-3d', transform: `rotateX(16deg) rotateY(${-12 + l * 1.8}deg) translateZ(${-60 + l * 20}px)`}}>
        {cards}
        <div style={{position: 'absolute', left: bx - 300, top: -30, width: 300, height: 620, background: 'linear-gradient(90deg, transparent, rgba(245,158,11,.28))', opacity: beamOn, transform: 'translateZ(30px)'}} />
        <div style={{position: 'absolute', left: bx - 3, top: -40, width: 6, height: 640, background: 'linear-gradient(transparent, #fff 15%, #fff 85%, transparent)', boxShadow: `0 0 50px 12px ${AMBER}`, opacity: beamOn, transform: 'translateZ(40px)'}} />
        <div style={{position: 'absolute', left: 0, top: 575, fontSize: 30, letterSpacing: 4, color: scanned === 10 ? AMBER : '#969dab', fontVariantNumeric: 'tabular-nums', opacity: pr(l, 1.2, 0.4), transform: 'translateZ(20px)'}}>
          分析进度 {scanned} / 10
        </div>
      </div>
    </div>
  );
};

// ---------- S3: matchup ----------
const LANES: [string, number, number, string, string][] = [
  ['上路', 78, 52, '上等马 · 可压制', '#fbbf24'],
  ['打野', 61, 70, '下等马 · 保下路', '#fb7185'],
  ['中路', 83, 66, '硬骨头 · 好抓', '#7dd3fc'],
];

const Matchups: React.FC<{l: number}> = ({l}) => {
  const vs = pb(l, 0.6, 0.5);
  const [sx, sy] = shake(l, 0.6, 18);
  return (
    <>
      <div style={{position: 'absolute', left: 1380, top: 90, width: 440, display: 'flex', alignItems: 'center', justifyContent: 'space-between', transform: `translate(${sx}px,${sy}px)`}}>
        <span style={{fontSize: 32, fontWeight: 700, color: '#60a5fa', opacity: vs, textShadow: '0 0 24px #3b82f6'}}>我方</span>
        <span style={{fontSize: 130, fontWeight: 900, fontStyle: 'italic', lineHeight: 1, opacity: Math.min(1, vs * 3), transform: `scale(${3.2 - 2.2 * vs})`, color: '#fde68a', filter: `drop-shadow(0 0 30px ${AMBER}aa)`}}>VS</span>
        <span style={{fontSize: 32, fontWeight: 700, color: '#fb7185', opacity: vs, textShadow: '0 0 24px #f43f5e'}}>敌方</span>
      </div>
      <Ring age={l - 0.65} x={1600} y={160} size={700} life={0.9} />
      <Burst age={l - 0.65} x={1600} y={160} n={30} power={420} seed={21} size={8} life={1.1} />
      {LANES.map(([name, mine, theirs, verdict, color], i) => {
        const s = 0.9 + i * 0.55;
        const top = 360 + i * 210;
        const p = pr(l, s, 0.6);
        const g = pr(l, s + 0.2, 1.0);
        const vp = pb(l, s + 1.3, 0.35);
        const lenL = (mine / 100) * 600 * g;
        const lenR = (theirs / 100) * 600 * g;
        const mineWins = mine >= theirs;
        const bar = {position: 'absolute', top: 58, height: 34, borderRadius: 17} as const;
        const head = (x: number, c: string) => (
          <div style={{position: 'absolute', left: x - 12, top: 64, width: 24, height: 24, borderRadius: '50%', background: '#fff', boxShadow: `0 0 30px 10px ${c}`, opacity: g > 0 && g < 1 ? 1 : 0.0}} />
        );
        return (
          <React.Fragment key={name}>
            <div style={{position: 'absolute', left: 210, top, width: 1500, height: 190, borderRadius: 24, background: 'linear-gradient(145deg, rgba(255,255,255,.07), rgba(255,255,255,.015))', border: '1.5px solid rgba(255,255,255,.1)', opacity: p, transform: `translateY(${(1 - p) * 60}px) scaleX(${0.9 + 0.1 * p})`}}>
              <div style={{...bar, left: 690 - lenL, width: lenL, background: 'linear-gradient(90deg,#bfdbfe,#3b82f6 45%,#1e3a8a)', boxShadow: `0 0 ${mineWins ? 40 : 14}px #3b82f6`}} />
              <div style={{...bar, left: 810, width: lenR, background: 'linear-gradient(270deg,#fecdd3,#f43f5e 45%,#881337)', boxShadow: `0 0 ${mineWins ? 14 : 40}px #f43f5e`}} />
              {head(690 - lenL, '#3b82f6')}
              {head(810 + lenR, '#f43f5e')}
              <div style={{position: 'absolute', left: 695, top: 18, width: 110, height: 110, borderRadius: '50%', background: '#0f1218', border: '2px solid #2a2f3a', display: 'grid', placeItems: 'center', fontSize: 36, fontWeight: 800, color: '#eceef2', boxShadow: `0 0 ${20 + 20 * Math.sin(l * 4 + i)}px rgba(245,158,11,.35)`}}>
                {name}
              </div>
              <div style={{position: 'absolute', left: 26, top: 38, fontSize: 72, fontWeight: 800, color: '#60a5fa', fontVariantNumeric: 'tabular-nums', textShadow: '0 0 30px #3b82f688'}}>{Math.round(mine * g)}</div>
              <div style={{position: 'absolute', right: 26, top: 38, fontSize: 72, fontWeight: 800, color: '#fb7185', fontVariantNumeric: 'tabular-nums', textShadow: '0 0 30px #f43f5e88'}}>{Math.round(theirs * g)}</div>
              <div style={{position: 'absolute', left: 0, right: 0, top: 136, textAlign: 'center'}}>
                <span style={{display: 'inline-block', fontSize: 28, fontWeight: 800, letterSpacing: 4, color, padding: '6px 26px', borderRadius: 999, border: `2px solid ${color}`, background: `${color}22`, opacity: Math.min(1, vp * 3), transform: `scale(${2.4 - 1.4 * vp}) rotate(${(1 - vp) * -8 - 2}deg)`, boxShadow: `0 0 ${(1 - vp) * 60 + 12}px ${color}88`}}>
                  {verdict}
                </span>
              </div>
            </div>
            <Ring age={l - (s + 1.3)} x={960} y={top + 158} size={560} color={color} life={0.6} w={4} />
            <Burst age={l - (s + 1.3)} x={960} y={top + 158} n={16} power={300} seed={50 + i * 20} size={7} life={0.9} color={color} />
          </React.Fragment>
        );
      })}
    </>
  );
};

// ---------- S4: automation + push ----------
const TOGGLES: [string, string][] = [
  ['自动接受对局', '延迟可调，随时取消'],
  ['自动 BP · 分路预设', '你手动选了就不替你换'],
  ['自动荣誉点赞', '对局结束后自动完成'],
];

const Automation: React.FC<{l: number}> = ({l}) => {
  const ph = pr(l, 0.7, 1.0);
  const vib = l > 2.6 ? Math.sin(l * 120) * 10 * Math.exp(-(l - 2.6) * 6) : 0;
  const notes: [number, number, string, string][] = [
    [2.6, 230, 'Lumina · 对局已找到', '排位赛 · 单双排，请回到客户端'],
    [3.3, 370, 'Lumina · 进入选人', '队友战绩已就绪，点击查看'],
  ];
  return (
    <>
      <Headline title="自动化" parts={[['省下的每一次点击，', false], ['都是专注', true]]} t={l} />
      {TOGGLES.map(([label, sub], i) => {
        const ton = 1.0 + i * 0.5;
        const p = pr(l, 0.4 + i * 0.25, 0.6);
        const on = pr(l, ton, 0.25);
        const fl = l > ton ? Math.exp(-(l - ton) * 3) : 0;
        const top = 380 + i * 200;
        return (
          <React.Fragment key={label}>
            <div style={{position: 'absolute', left: 140, top, width: 860, height: 150, padding: '0 44px', borderRadius: 24, display: 'flex', alignItems: 'center', justifyContent: 'space-between', background: 'linear-gradient(145deg, rgba(255,255,255,.08), rgba(255,255,255,.015))', border: `1.5px solid color-mix(in srgb, ${AMBER} ${fl * 100}%, rgba(255,255,255,.1))`, boxShadow: `0 0 ${fl * 60}px rgba(245,158,11,${fl * 0.7})`, opacity: p, transform: `translateX(${(1 - p) * -240}px)`}}>
              <div>
                <div style={{fontSize: 42, fontWeight: 700}}>{label}</div>
                <div style={{fontSize: 24, color: '#969dab', marginTop: 6}}>{sub}</div>
              </div>
              <div style={{width: 100, height: 56, borderRadius: 28, position: 'relative', background: `color-mix(in srgb, ${AMBER} ${on * 100}%, #2a2f3a)`, boxShadow: `0 0 ${on * 30}px ${AMBER}88`}}>
                <i style={{position: 'absolute', top: 6, left: 6, width: 44, height: 44, borderRadius: '50%', background: on > 0.5 ? '#fff' : '#969dab', transform: `translateX(${on * 44}px)`}} />
              </div>
            </div>
            <Ring age={l - ton} x={906} y={top + 75} size={420} color={AMBER} life={0.8} w={4} />
            <Burst age={l - ton} x={906} y={top + 75} n={18} power={240} seed={90 + i * 17} size={7} life={0.9} color="#fde68a" />
          </React.Fragment>
        );
      })}
      <Ring age={l - 2.6} x={1460} y={560} size={900} life={1.1} color={AMBER} />
      <div style={{position: 'absolute', left: 1180, top: 200, width: 560, height: 700, perspective: 1800}}>
        <div style={{position: 'absolute', left: 0, top: 0, width: 560, height: 700, opacity: ph, transform: `translate(${(1 - ph) * 320 + vib}px, ${Math.sin(l * 1.8) * 10}px) rotateY(${-22 + Math.sin(l * 0.9) * 4}deg) rotateX(6deg)`}}>
          <div style={{position: 'absolute', inset: 0, borderRadius: 60, border: '3px solid #3d4351', background: 'linear-gradient(160deg,#1a1f2b,#0b0d12)', boxShadow: `0 50px 100px rgba(0,0,0,.6), 0 0 ${40 + 40 * Math.exp(-Math.max(0, l - 2.6) * 2)}px rgba(245,158,11,.3)`, overflow: 'hidden'}}>
            <div style={{position: 'absolute', left: 190, top: 20, width: 180, height: 38, borderRadius: 20, background: '#000'}} />
            <div style={{position: 'absolute', left: 0, right: 0, top: 90, textAlign: 'center', fontSize: 120, fontWeight: 300, letterSpacing: 2}}>21:08</div>
            <div style={{position: 'absolute', left: 0, right: 0, top: 250, textAlign: 'center', fontSize: 26, color: '#969dab'}}>10月1日 星期四</div>
            {notes.map(([nt, top, title, body]) => {
              const np = pb(l, nt, 0.5);
              return (
                <div key={title} style={{position: 'absolute', left: 28, right: 28, top: top + 40, padding: '22px 24px', borderRadius: 26, background: 'rgba(40,46,60,.92)', display: 'flex', gap: 18, alignItems: 'center', opacity: Math.min(1, np * 3), transform: `translateY(${(1 - np) * -220}px) scale(${0.9 + 0.1 * np})`, boxShadow: '0 12px 30px rgba(0,0,0,.4)'}}>
                  <div style={{width: 64, height: 64, borderRadius: 16, flexShrink: 0, background: 'linear-gradient(135deg,#fde68a,#f59e0b)', display: 'grid', placeItems: 'center', fontSize: 38, fontWeight: 800, color: '#14171e'}}>L</div>
                  <div>
                    <div style={{fontSize: 27, fontWeight: 700, color: '#fbbf24'}}>{title}</div>
                    <div style={{fontSize: 22, color: '#c1c6d0', marginTop: 4}}>{body}</div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </>
  );
};

// ---------- S5: rank chart ----------
const LP = [40, 70, 60, 130, 110, 190, 170, 150, 260, 240, 330, 300, 390, 370, 470];
const CHART = LP.map((v, i) => ({x: (i * 1640) / (LP.length - 1), y: 540 - v}));
const LINE_D = CHART.map((p, i) => `${i ? 'L' : 'M'}${p.x.toFixed(1)} ${p.y}`).join(' ');
const SEG = CHART.slice(1).map((p, i) => Math.hypot(p.x - CHART[i].x, p.y - CHART[i].y));
const CUM = [0, ...SEG.map((_, i) => SEG.slice(0, i + 1).reduce((a, b) => a + b, 0))];
const TOTAL = CUM[CUM.length - 1];

const pointAt = (len: number) => {
  const d = cl(len, 0, TOTAL);
  for (let i = 0; i < SEG.length; i++) {
    if (d <= CUM[i + 1]) {
      const k = (d - CUM[i]) / SEG[i];
      return {x: CHART[i].x + (CHART[i + 1].x - CHART[i].x) * k, y: CHART[i].y + (CHART[i + 1].y - CHART[i].y) * k};
    }
  }
  return CHART[CHART.length - 1];
};

const CH_X = 140;
const CH_Y = 380;
const Trend: React.FC<{l: number}> = ({l}) => {
  const g = pr(l, 0.5, 2.2);
  const head = pointAt(TOTAL * g);
  const done = l - 2.75;
  const gain = Math.round(126 * g);
  const badge = pb(l, 2.75, 0.4);
  return (
    <>
      <Headline title="段位记录" parts={[['每一局胜点，', false], ['画成走势', true]]} t={l} />
      <div style={{position: 'absolute', right: 150, top: 100, textAlign: 'right', opacity: pr(l, 0.4, 0.5)}}>
        <div style={{fontSize: 26, letterSpacing: 6, color: '#969dab'}}>本赛季胜点</div>
        <div style={{fontSize: 120, fontWeight: 800, lineHeight: 1.05, color: AMBER, fontVariantNumeric: 'tabular-nums', textShadow: `0 0 50px ${AMBER}99`}}>+{gain}</div>
      </div>
      <svg style={{position: 'absolute', left: CH_X, top: CH_Y, width: 1640, height: 560, overflow: 'visible'}} viewBox="0 0 1640 560">
        <g stroke="rgba(255,255,255,.1)" strokeWidth={2}>
          {[0, 180, 360, 540].map((y) => (
            <line key={y} x1={0} y1={y} x2={1640} y2={y} />
          ))}
        </g>
        <defs>
          <linearGradient id="g" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" stopColor={AMBER} stopOpacity={0.4} />
            <stop offset="1" stopColor={AMBER} stopOpacity={0} />
          </linearGradient>
        </defs>
        <path d={`${LINE_D} L1640 540 L0 540 Z`} fill="url(#g)" style={{clipPath: `inset(0 ${(1 - head.x / 1640) * 100}% 0 0)`}} />
        <path d={LINE_D} fill="none" stroke={AMBER} strokeWidth={26} strokeLinecap="round" strokeLinejoin="round" strokeDasharray={TOTAL} strokeDashoffset={TOTAL * (1 - g)} opacity={0.35} style={{filter: 'blur(14px)'}} />
        <path d={LINE_D} fill="none" stroke="#fde68a" strokeWidth={7} strokeLinecap="round" strokeLinejoin="round" strokeDasharray={TOTAL} strokeDashoffset={TOTAL * (1 - g)} />
        {CHART.map((p, k) => {
          const np = cl((g * TOTAL - CUM[k]) / 60);
          return np > 0 ? <circle key={k} cx={p.x} cy={p.y} r={9 * np} fill="#14171e" stroke="#fff" strokeWidth={4} /> : null;
        })}
        {Array.from({length: 22}, (_, k) => {
          const len = TOTAL * g - (k + 1) * 14;
          if (len < 0 || g >= 1) return null;
          const p = pointAt(len);
          const f = 1 - k / 22;
          return <circle key={k} cx={p.x} cy={p.y + (rand(k + 5) - 0.5) * 22 * (1 - f)} r={9 * f} fill={k % 2 ? '#fff' : AMBER} opacity={f * 0.8} />;
        })}
        {g < 1 ? <circle cx={head.x} cy={head.y} r={34 + 6 * Math.sin(l * 14)} fill="none" stroke="#fde68a" strokeWidth={2} opacity={0.6} /> : null}
        <circle cx={head.x} cy={head.y} r={16} fill="#fff" style={{filter: `drop-shadow(0 0 18px ${AMBER})`}} />
      </svg>
      <Ring age={done} x={CH_X + head.x} y={CH_Y + head.y} size={700} life={0.9} />
      <Burst age={done} x={CH_X + head.x} y={CH_Y + head.y} n={44} power={520} seed={133} size={9} life={1.3} />
      <div style={{position: 'absolute', left: 1210, top: 250, fontSize: 40, fontWeight: 800, letterSpacing: 6, color: '#14171e', padding: '10px 34px', borderRadius: 999, background: 'linear-gradient(90deg,#fde68a,#f59e0b)', opacity: Math.min(1, badge * 3), transform: `scale(${2.2 - 1.2 * badge}) rotate(${(1 - badge) * 10}deg)`, boxShadow: `0 0 ${50 + (1 - badge) * 60}px ${AMBER}`}}>
        段位上升 ↑
      </div>
    </>
  );
};

// ---------- S6: outro ----------
const Outro: React.FC<{l: number}> = ({l}) => {
  const [sx, sy] = shake(l, 0.15, 22, 0.6);
  const age = l - 0.1;
  return (
    <>
      <div style={{position: 'absolute', left: 960 - 1200, top: 540 - 1200, width: 2400, height: 2400, opacity: pr(l, 0, 0.8) * 0.9, transform: `rotate(${l * 7}deg)`, background: 'repeating-conic-gradient(from 0deg, rgba(245,158,11,.22) 0deg 4deg, transparent 4deg 12deg)', maskImage: 'radial-gradient(circle, black 5%, transparent 45%)', WebkitMaskImage: 'radial-gradient(circle, black 5%, transparent 45%)'}} />
      <div style={{position: 'absolute', left: 360, top: 140, width: 1200, height: 800, background: 'radial-gradient(closest-side, rgba(245,158,11,.4), transparent)', opacity: pr(l, 0, 0.6) * (0.85 + 0.15 * Math.sin(l * 3))}} />
      <Ring age={age} x={960} y={500} size={1900} />
      <Ring age={age - 0.15} x={960} y={500} size={1300} color={AMBER} />
      <Burst age={age} x={960} y={500} n={90} power={1000} seed={301} size={9} />
      <div style={{position: 'absolute', inset: 0, transform: `translate(${sx}px,${sy}px)`}}>
        <Logo t={l} start={0.05} band={-1.5 + 8.5 * lin(l, 0.9, 0.9)} top={290} size={220} from={3.2} />
      </div>
      <div style={{position: 'absolute', left: 0, right: 0, top: 590, textAlign: 'center', fontSize: 46, letterSpacing: 6, color: '#eceef2'}}>
        <Chars text="不读游戏内存 · 不注入进程" t={l} start={0.7} stagger={0.035} dur={0.5} y={24} blur={10} scale={1} />
      </div>
      <div style={{position: 'absolute', left: 0, right: 0, top: 680, textAlign: 'center', fontSize: 34, letterSpacing: 6, color: '#969dab'}}>
        <Chars text="开源 · MIT · 免费使用" t={l} start={1.2} stagger={0.035} dur={0.5} y={20} blur={8} scale={1} />
      </div>
    </>
  );
};

export const Promo: React.FC = () => {
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const t = frame / fps;
  const flash = FLASHES.reduce((a, [b, p]) => a + p * Math.exp(-Math.pow((t - b) / 0.12, 2)), 0);

  return (
    <AbsoluteFill style={{background: 'radial-gradient(ellipse at 50% 120%,#161a24,#05060a 65%)', fontFamily: FONT, color: '#eceef2', overflow: 'hidden'}}>
      <Audio src={staticFile('soundtrack.wav')} />
      <Backdrop t={t} />
      <Scene i={0} t={t}>
        <Intro t={t} />
      </Scene>
      <Scene i={1} t={t}>
        <Headline title="对局面板" parts={[['选人加载，', false], ['十人底细', true], ['一眼看穿', false]]} t={t - START[1]} />
        <Roster l={t - START[1]} />
      </Scene>
      <Scene i={2} t={t}>
        <Headline title="田忌赛马" parts={[['战力对位，', false], ['针对建议', true], ['直接给出', false]]} t={t - START[2]} />
        <Matchups l={t - START[2]} />
      </Scene>
      <Scene i={3} t={t}>
        <Automation l={t - START[3]} />
      </Scene>
      <Scene i={4} t={t}>
        <Trend l={t - START[4]} />
      </Scene>
      <Scene i={5} t={t}>
        <Outro l={t - START[5]} />
      </Scene>
      <AbsoluteFill style={{background: 'radial-gradient(circle at 50% 50%, #fff 0%, #fde68a 35%, transparent 75%)', opacity: Math.min(1, flash), mixBlendMode: 'screen'}} />
      <AbsoluteFill style={{background: 'radial-gradient(ellipse at 50% 50%, transparent 55%, rgba(0,0,0,.6) 100%)'}} />
    </AbsoluteFill>
  );
};
