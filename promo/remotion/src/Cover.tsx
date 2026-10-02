import React from 'react';
import {AbsoluteFill} from 'remotion';
import {AMBER, Backdrop, FONT, Logo, TONE} from './Promo';

type Tone = keyof typeof TONE;

const PlayerCard: React.FC<{name: string; rank: string; team: string; chips: [string, Tone][]; top: number; glow?: boolean}> = ({name, rank, team, chips, top, glow}) => (
  <div
    style={{
      position: 'absolute',
      left: 0,
      top,
      width: 700,
      height: 250,
      padding: 30,
      borderRadius: 30,
      background: 'linear-gradient(145deg, rgba(255,255,255,.13), rgba(255,255,255,.03))',
      border: `2px solid ${glow ? AMBER : 'rgba(255,255,255,.18)'}`,
      boxShadow: `0 40px 80px rgba(0,0,0,.55)${glow ? `, 0 0 90px rgba(245,158,11,.55)` : ''}`,
    }}
  >
    <div style={{display: 'flex', alignItems: 'center', gap: 24}}>
      <div style={{width: 90, height: 90, borderRadius: '50%', background: `linear-gradient(135deg, ${team}, #1d2129)`, display: 'grid', placeItems: 'center', fontSize: 42, fontWeight: 800, boxShadow: `0 0 30px ${team}88`}}>{name[0]}</div>
      <div>
        <div style={{fontSize: 46, fontWeight: 800}}>{name}</div>
        <div style={{fontSize: 28, color: '#a8afbd', marginTop: 4}}>{rank}</div>
      </div>
    </div>
    <div style={{display: 'flex', gap: 14, marginTop: 28}}>
      {chips.map(([label, tone]) => (
        <span key={label} style={{fontSize: 32, fontWeight: 700, padding: '8px 22px', borderRadius: 999, background: TONE[tone].bg, color: TONE[tone].fg, boxShadow: `0 0 28px ${TONE[tone].fg}55`}}>
          {label}
        </span>
      ))}
    </div>
  </div>
);

const MatchCard: React.FC<{top: number}> = ({top}) => (
  <div style={{position: 'absolute', left: 0, top, width: 700, height: 250, borderRadius: 30, background: 'linear-gradient(145deg, rgba(255,255,255,.13), rgba(255,255,255,.03))', border: '2px solid rgba(255,255,255,.18)', boxShadow: '0 40px 80px rgba(0,0,0,.55)'}}>
    <div style={{position: 'absolute', left: 36, top: 38, fontSize: 84, fontWeight: 800, color: '#60a5fa', textShadow: '0 0 36px #3b82f6aa'}}>78</div>
    <div style={{position: 'absolute', right: 36, top: 38, fontSize: 84, fontWeight: 800, color: '#fb7185', textShadow: '0 0 36px #f43f5eaa'}}>52</div>
    <div style={{position: 'absolute', left: 340 - 190, top: 66, width: 190, height: 40, borderRadius: 20, background: 'linear-gradient(90deg,#bfdbfe,#3b82f6 45%,#1e3a8a)', boxShadow: '0 0 40px #3b82f6'}} />
    <div style={{position: 'absolute', left: 360, top: 66, width: 130, height: 40, borderRadius: 20, background: 'linear-gradient(270deg,#fecdd3,#f43f5e 45%,#881337)', boxShadow: '0 0 20px #f43f5e'}} />
    <div style={{position: 'absolute', left: 300, top: 40, width: 100, height: 100, borderRadius: '50%', background: '#0f1218', border: '3px solid #3d4351', display: 'grid', placeItems: 'center', fontSize: 34, fontWeight: 800}}>上路</div>
    <div style={{position: 'absolute', left: 0, right: 0, top: 160, textAlign: 'center'}}>
      <span style={{display: 'inline-block', fontSize: 40, fontWeight: 800, letterSpacing: 6, color: '#fbbf24', padding: '8px 36px', borderRadius: 999, border: '3px solid #fbbf24', background: '#fbbf2422', transform: 'rotate(-2deg)', boxShadow: '0 0 40px #fbbf2488'}}>
        上等马 · 可压制
      </span>
    </div>
  </div>
);

const pill: React.CSSProperties = {fontSize: 34, fontWeight: 700, padding: '10px 30px', borderRadius: 999, border: '2px solid rgba(255,255,255,.28)', background: 'rgba(255,255,255,.07)', color: '#eceef2'};

export const Cover: React.FC = () => (
  <AbsoluteFill style={{background: 'radial-gradient(ellipse at 70% 40%,#1d2230,#05060a 70%)', fontFamily: FONT, color: '#eceef2', overflow: 'hidden'}}>
    <Backdrop t={7.3} />
    {/* rays + glow behind the cards */}
    <div style={{position: 'absolute', left: 1430 - 1300, top: 560 - 1300, width: 2600, height: 2600, transform: 'rotate(8deg)', background: 'repeating-conic-gradient(from 0deg, rgba(245,158,11,.2) 0deg 4deg, transparent 4deg 12deg)', maskImage: 'radial-gradient(circle, black 5%, transparent 42%)', WebkitMaskImage: 'radial-gradient(circle, black 5%, transparent 42%)'}} />
    <div style={{position: 'absolute', left: 1000, top: 140, width: 900, height: 900, background: 'radial-gradient(closest-side, rgba(245,158,11,.35), transparent)'}} />

    {/* left: title block */}
    <div style={{position: 'absolute', left: 100, top: 96}}>
      <span style={{...pill, color: AMBER, borderColor: AMBER, background: 'rgba(245,158,11,.12)', letterSpacing: 4}}>英雄联盟 · 战绩助手</span>
    </div>
    <div style={{position: 'absolute', left: 60, top: 190, width: 1000, height: 260}}>
      <Logo t={10} start={0} band={-9} top={0} size={210} from={1} />
    </div>
    <div style={{position: 'absolute', left: 100, top: 470, fontSize: 210, fontWeight: 900, lineHeight: 1.05, letterSpacing: 4, whiteSpace: 'nowrap', color: AMBER, textShadow: `0 0 80px ${AMBER}aa, 0 8px 0 rgba(0,0,0,.35)`}}>十人底细</div>
    <div style={{position: 'absolute', left: 100, top: 700, fontSize: 210, fontWeight: 900, lineHeight: 1.05, letterSpacing: 4, whiteSpace: 'nowrap', color: '#fff', textShadow: '0 0 60px rgba(255,255,255,.35), 0 8px 0 rgba(0,0,0,.35)'}}>一眼看穿</div>
    <div style={{position: 'absolute', left: 104, top: 960, fontSize: 40, letterSpacing: 6, color: '#c1c6d0'}}>选人阶段 · 玩家标签 · 田忌赛马</div>
    <div style={{position: 'absolute', left: 100, top: 1062, display: 'flex', gap: 18}}>
      <span style={pill}>开源</span>
      <span style={pill}>免费</span>
      <span style={pill}>不读内存 · 不注入进程</span>
    </div>

    {/* right: tilted card stack */}
    <div style={{position: 'absolute', left: 1090, top: 150, width: 700, height: 900, perspective: 2400}}>
      <div style={{position: 'absolute', inset: 0, transformStyle: 'preserve-3d', transform: 'rotateY(-22deg) rotateX(8deg) rotateZ(2deg)'}}>
        <PlayerCard top={0} name="夜行者" rank="白银 II" team="#fb7185" chips={[['三连败', 'bad'], ['闪现异位', 'bad']]} />
        <PlayerCard top={290} name="Rainfall" rank="钻石 IV" team="#60a5fa" chips={[['连胜 7', 'good'], ['绝活 · 劫', 'good']]} glow />
        <MatchCard top={580} />
      </div>
    </div>
  </AbsoluteFill>
);
