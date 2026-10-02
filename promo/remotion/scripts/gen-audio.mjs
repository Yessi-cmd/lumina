// Synthesises the promo soundtrack (pad + pulse + hits + UI blips) into public/soundtrack.wav.
// Event times mirror the scene timeline in src/Promo.tsx.
import fs from 'node:fs';

const SR = 44100;
const DUR = 22;
const N = SR * DUR;
const L = new Float32Array(N);
const R = new Float32Array(N);
const TAU = Math.PI * 2;

let seed = 1234567;
const rnd = () => {
  seed = (seed * 1664525 + 1013904223) >>> 0;
  return seed / 4294967296;
};

function add(t0, dur, fn, pan = 0) {
  const i0 = Math.floor(t0 * SR);
  const n = Math.floor(dur * SR);
  const gl = pan > 0 ? 1 - pan : 1;
  const gr = pan < 0 ? 1 + pan : 1;
  for (let i = 0; i < n; i++) {
    const idx = i0 + i;
    if (idx < 0 || idx >= N) continue;
    const v = fn(i / SR);
    L[idx] += v * gl;
    R[idx] += v * gr;
  }
}

const boom = (t0, amp) => {
  add(t0, 2.0, (t) => amp * Math.exp(-t * 2.4) * Math.sin(TAU * (36 * t + (150 * (1 - Math.exp(-6 * t))) / 6)));
  let y = 0;
  add(t0, 0.3, (t) => {
    y += 0.3 * (rnd() * 2 - 1 - y);
    return amp * 0.9 * Math.exp(-t * 16) * y;
  });
};

const kick = (t0, amp) =>
  add(t0, 0.35, (t) => amp * Math.exp(-t * 11) * Math.sin(TAU * (45 * t + (110 * (1 - Math.exp(-30 * t))) / 30)));

const hat = (t0, amp) => {
  let y = 0;
  add(t0, 0.06, (t) => {
    const x = rnd() * 2 - 1;
    y += 0.6 * (x - y);
    return amp * Math.exp(-t * 70) * (x - y);
  }, 0.3);
};

const riser = (t1, dur, amp) => {
  let y = 0;
  add(t1 - dur, dur, (t) => {
    const x = rnd() * 2 - 1;
    y += 0.45 * (x - y);
    const k = t / dur;
    return amp * k * k * (x - y);
  });
};

const ping = (t0, f, amp, decay, pan = 0) =>
  add(t0, decay * 5, (t) => amp * Math.exp(-t / decay) * (1 - Math.exp(-t * 3000)) * Math.sin(TAU * f * t), pan);

const click = (t0) => {
  let y = 0;
  add(t0, 0.05, (t) => {
    y += 0.5 * (rnd() * 2 - 1 - y);
    return 0.2 * Math.exp(-t * 120) * y;
  });
  ping(t0, 2400, 0.07, 0.02);
};

// ---- pad ----
const CHORDS = [
  [0, 7.3, [110, 164.81, 220, 261.63, 329.63]],
  [7, 11.3, [87.31, 130.81, 174.61, 220, 261.63]],
  [11, 15.3, [130.81, 196, 261.63, 329.63, 392]],
  [15, 18.3, [98, 146.83, 196, 246.94, 293.66]],
  [18, 22.2, [110, 164.81, 220, 261.63, 329.63, 440]],
];
for (const [t0, t1, notes] of CHORDS) {
  const T = t1 - t0 + 1;
  for (const f of notes) {
    for (const [det, pan] of [[1.002, -0.6], [0.998, 0.6]]) {
      add(t0, T, (t) => {
        const env = Math.max(0, Math.min(t / 1.0, (T - t) / 1.0, 1));
        const trem = 0.85 + 0.15 * Math.sin(TAU * 0.25 * t);
        return 0.03 * env * trem * (Math.sin(TAU * f * det * t) + 0.25 * Math.sin(TAU * 2 * f * det * t));
      }, pan);
    }
  }
}

// ---- pulse: kick on the beat, hat off-beat, 8th-note plucks (120 bpm) ----
const chordAt = (t) => CHORDS.filter(([a, b]) => t >= a && t < b).pop() ?? CHORDS[CHORDS.length - 1];
for (let t = 3.0; t < 17.9; t += 0.5) {
  kick(t, 0.2);
  hat(t + 0.25, 0.05);
}
for (let k = 0, t = 3.0; t < 18.0; t += 0.25, k++) {
  const notes = chordAt(t)[2];
  const f = notes[(k * 3) % notes.length] * 2;
  ping(t, f, 0.035, 0.12, k % 2 ? 0.4 : -0.4);
}

// ---- hits at scene boundaries ----
riser(0.88, 0.9, 0.2);
boom(0.88, 0.8);
for (const t of [3.0, 7.0, 11.0, 15.0]) {
  riser(t, 0.5, 0.16);
  boom(t, 0.55);
}
riser(18.0, 0.8, 0.22);
boom(18.0, 1.0);

// ---- intro sparkles ----
[1318.5, 1568, 1760, 2093, 2349, 1760, 2637, 3136].forEach((f, i) => ping(1.0 + i * 0.16, f, 0.05, 0.35, i % 2 ? 0.5 : -0.5));

// ---- scene 2: scan ticks (match chip timing in Promo.tsx) ----
const CHIPS = [2, 2, 1, 2, 2, 2, 1, 1, 2, 2];
CHIPS.forEach((n, i) => {
  const cx = (i % 5) * 336 + 150;
  const tc = 3.0 + 1.4 + (2.0 * (cx + 80)) / 1804;
  for (let j = 0; j < n; j++) ping(tc + 0.1 + 0.12 * j, 1500 + (i % 5) * 220 + j * 330, 0.07, 0.06, ((i % 5) - 2) * 0.3);
});

// ---- scene 3: VS slam, bars, verdict stamps ----
boom(7.6, 0.6);
for (let i = 0; i < 3; i++) {
  const s = 7 + 0.9 + i * 0.55;
  add(s + 0.2, 1.0, (t) => 0.035 * Math.sin(TAU * (220 * t + 180 * t * t)) * Math.min(1, t * 8) * Math.min(1, (1 - t) * 8));
  boom(s + 1.3, 0.3);
  ping(s + 1.3, 880, 0.08, 0.2);
}

// ---- scene 4: toggle clicks, notifications ----
for (let i = 0; i < 3; i++) {
  click(11 + 1.0 + i * 0.5);
  ping(11 + 1.0 + i * 0.5, 880 + i * 220, 0.06, 0.25);
}
const vibrate = (t0) => add(t0, 0.45, (t) => 0.13 * Math.exp(-t * 5) * Math.sin(TAU * 70 * t) * (0.6 + 0.4 * Math.sin(TAU * 28 * t)));
for (const [t, f] of [[13.6, 1318.5], [14.3, 1046.5]]) {
  vibrate(t);
  ping(t, f, 0.14, 0.5);
  ping(t, f * 1.5, 0.08, 0.5);
}

// ---- scene 5: chart climb ----
add(15.5, 2.4, (t) => 0.05 * Math.sin(TAU * (330 * t + (550 * t * t) / (2 * 2.4))) * Math.min(1, t * 4) * Math.min(1, (2.4 - t) * 4));
for (let k = 0; k < 15; k++) ping(15.5 + (k / 14) * 2.0, 600 + k * 55, 0.045, 0.08, Math.sin(k) * 0.4);
for (const f of [659.25, 880, 1046.5, 1318.5]) ping(17.75, f, 0.07, 0.9);
boom(17.75, 0.35);

// ---- outro shimmer ----
[880, 1318.5, 1760, 2093, 2637].forEach((f, i) => ping(18.2 + i * 0.12, f, 0.06, 0.6, i % 2 ? 0.5 : -0.5));

// ---- master ----
let peak = 0;
for (let i = 0; i < N; i++) {
  const t = i / SR;
  const fade = Math.min(1, Math.max(0, (DUR - t) / 0.8)) * Math.min(1, t / 0.05);
  L[i] *= fade;
  R[i] *= fade;
  peak = Math.max(peak, Math.abs(L[i]), Math.abs(R[i]));
}
const g = 0.89 / peak;
const buf = Buffer.alloc(44 + N * 4);
buf.write('RIFF', 0);
buf.writeUInt32LE(36 + N * 4, 4);
buf.write('WAVEfmt ', 8);
buf.writeUInt32LE(16, 16);
buf.writeUInt16LE(1, 20);
buf.writeUInt16LE(2, 22);
buf.writeUInt32LE(SR, 24);
buf.writeUInt32LE(SR * 4, 28);
buf.writeUInt16LE(4, 32);
buf.writeUInt16LE(16, 34);
buf.write('data', 36);
buf.writeUInt32LE(N * 4, 40);
for (let i = 0; i < N; i++) {
  buf.writeInt16LE(Math.round(Math.tanh(L[i] * g * 1.2) * 32000), 44 + i * 4);
  buf.writeInt16LE(Math.round(Math.tanh(R[i] * g * 1.2) * 32000), 46 + i * 4);
}
fs.mkdirSync('public', {recursive: true});
fs.writeFileSync('public/soundtrack.wav', buf);
console.log('wrote public/soundtrack.wav', (buf.length / 1e6).toFixed(1) + 'MB');
