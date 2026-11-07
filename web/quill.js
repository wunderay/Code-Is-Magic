// The quill writes an already-rendered diagram onto the page, seal by seal.
// Display only: it clips the finished drawing and never changes what is exported.
import { pointOnChannel } from './flow.js';

const TAU = Math.PI * 2;
const FEATHER = 0.5; // radians of wet, half-dried ink behind the nib
const clamp = (value, low, high) => Math.max(low, Math.min(high, value));
const ease = t => t < 0.5 ? 2 * t * t : 1 - (-2 * t + 2) ** 2 / 2;
const hash = id => { let h = (Number(id) + 1) * 2654435761; h ^= h >>> 15; return (h >>> 0) / 4294967296; };

// Seals are written in the order the call rehearsal first visits them, so a caller
// is inked just before the spells it invokes. Unvisited seals follow, nearest first.
export function writingOrder(placements, lanes = []) {
  const first = new Map();
  for (const lane of lanes) for (const event of lane.events || []) {
    if (event.kind === 'seal' && (!first.has(event.id) || event.start < first.get(event.id))) first.set(event.id, event.start);
  }
  let cx = 0, cy = 0;
  for (const p of placements) { cx += p.x / placements.length; cy += p.y / placements.length; }
  return [...placements].sort((a, b) => {
    const ta = first.has(a.id) ? first.get(a.id) : Infinity, tb = first.has(b.id) ? first.get(b.id) : Infinity;
    if (ta !== tb) return ta - tb;
    return Math.hypot(a.x - cx, a.y - cy) - Math.hypot(b.x - cx, b.y - cy) || a.id - b.id;
  });
}

// Times are in milliseconds from the first stroke. Many seals are written at once
// in overlapping waves, so 400 seals take about eight seconds and ten take three.
export function writingPlan(placements, channels = [], lanes = [], { fast = false } = {}) {
  const order = writingOrder(placements, lanes);
  const count = order.length, scale = fast ? 0.32 : 1;
  const largest = Math.max(1, ...order.map(p => p.radius || 0));
  const span = clamp(900 + count * 15, 900, 6400) * scale;
  const seals = new Map();
  order.forEach((p, rank) => {
    const duration = (420 + 680 * Math.sqrt(clamp((p.radius || 0) / largest, 0, 1))) * (fast ? 0.55 : 1);
    // A slow first flourish, then the hand quickens.
    const start = count > 1 ? span * (rank / (count - 1)) ** 1.18 : 0;
    seals.set(p.id, { seal: p, start, duration, angle: -Math.PI / 2 + (hash(p.id) - 0.5) * 1.1, rank });
  });
  const strokes = [];
  channels.forEach((channel, index) => {
    const a = seals.get(channel.from), b = seals.get(channel.to);
    if (!a || !b) return;
    // Once both seals are nearly inked, the pen leaves the caller and draws toward the callee.
    const start = Math.max(a.start + a.duration * 0.92, b.start + b.duration * 0.8);
    strokes.push({ index, start, duration: clamp(240 + channel.length * 0.5, 300, 950) * (fast ? 0.6 : 1) });
  });
  let total = 0;
  for (const s of seals.values()) total = Math.max(total, s.start + s.duration);
  for (const s of strokes) total = Math.max(total, s.start + s.duration);
  return { seals, strokes, total };
}

export class Quill {
  constructor(diagram, ink, { fast = false, now = performance.now() } = {}) {
    this.ink = ink;
    this.plan = writingPlan(diagram.placements.filter(p => p.radius > 0), ink.paths, ink.flow, { fast });
    this.begin = now;
    this.fast = fast;
    this.finished = false;
  }
  elapsed(now) { return now - this.begin; }
  done(now) { return this.finished || this.elapsed(now) >= this.plan.total; }
  finish() { this.finished = true; }
  // Fraction of seals fully inscribed, for the readout.
  written(now) {
    const t = this.elapsed(now);
    let count = 0;
    for (const s of this.plan.seals.values()) if (t >= s.start + s.duration) count++;
    return count;
  }
  paint(ctx, base, chan, view, now, light) {
    const { width, height, dpr, panX, panY, zoom } = view, t = this.elapsed(now);
    const inView = (x, y, r) => x + r > 0 && x - r < width && y + r > 0 && y - r < height;
    // 1 · Call channels: a ribbon clip grows along each braid from caller to callee.
    const ribbons = new Path2D(), nibs = [];
    const half = Math.max(5, 11 * zoom);
    for (const stroke of this.plan.strokes) {
      const p = (t - stroke.start) / stroke.duration;
      if (p <= 0) continue;
      const channel = this.ink.paths[stroke.index];
      this.ribbon(ribbons, channel, Math.min(1, ease(Math.min(1, p))), half, view);
      if (p < 1) nibs.push(pointOnChannel(channel, ease(p)));
    }
    if (chan) { ctx.save(); ctx.clip(ribbons); ctx.drawImage(chan, 0, 0, width, height); ctx.restore(); }
    // 2 · Seals: a pen sweep reveals the finished ink beneath a growing arc wedge.
    const wedges = new Path2D(), active = [];
    for (const s of this.plan.seals.values()) {
      const p = (t - s.start) / s.duration;
      if (p <= 0) continue;
      const x = panX + s.seal.x * zoom, y = panY + s.seal.y * zoom, r = s.seal.radius * zoom * 1.22;
      if (!inView(x, y, r)) continue;
      if (p >= 1) { wedges.moveTo(x + r, y); wedges.arc(x, y, r, 0, TAU); continue; }
      const sweep = ease(p) * (TAU + FEATHER);
      const solid = Math.min(TAU, sweep - FEATHER);
      if (solid > 0) { wedges.moveTo(x, y); wedges.arc(x, y, r, s.angle, s.angle + solid); wedges.closePath(); }
      active.push({ s, x, y, r, sweep, p });
    }
    ctx.save(); ctx.clip(wedges); ctx.drawImage(base, 0, 0, width, height); ctx.restore();
    // The wet edge: the newest ink is thinner, as if it has not yet soaked in.
    for (const a of active) {
      const sx = Math.max(0, Math.floor((a.x - a.r) * dpr)), sy = Math.max(0, Math.floor((a.y - a.r) * dpr));
      const sw = Math.min(base.width - sx, Math.ceil(a.r * 2 * dpr) + 2), sh = Math.min(base.height - sy, Math.ceil(a.r * 2 * dpr) + 2);
      if (sw < 1 || sh < 1) continue;
      for (let i = 0; i < 4; i++) {
        const from = Math.max(0, a.sweep - FEATHER + i * FEATHER / 4), to = Math.min(TAU, a.sweep - FEATHER + (i + 1) * FEATHER / 4);
        if (to <= from) continue;
        ctx.save(); ctx.globalAlpha = 0.78 - i * 0.2;
        ctx.beginPath(); ctx.moveTo(a.x, a.y); ctx.arc(a.x, a.y, a.r, a.s.angle + from, a.s.angle + to); ctx.closePath(); ctx.clip();
        ctx.drawImage(base, sx, sy, sw, sh, sx / dpr, sy / dpr, sw / dpr, sh / dpr);
        ctx.restore();
      }
    }
    // 3 · The pen itself: quills ride the largest sweeps, small nibs the rest.
    const ink = light ? '#3b2a1c' : '#f3e2b6';
    const glow = light ? 'rgba(80,50,20,0)' : 'rgba(240,205,130,.75)';
    active.sort((a, b) => b.r - a.r);
    active.forEach((a, i) => {
      const angle = a.s.angle + Math.min(TAU, a.sweep), rim = a.r * 0.84;
      const x = a.x + Math.cos(angle) * rim, y = a.y + Math.sin(angle) * rim;
      const fade = Math.min(1, a.p * 6, (1 - a.p) * 8);
      if (a.r > 9) this.bleed(ctx, x, y, Math.max(2.5, a.r * 0.07), light, fade);
      if (i < 7 && a.r > 9) this.quillGlyph(ctx, x, y, clamp(a.r * 0.95, 22, 62), ink, glow, fade, now + a.s.rank * 97, light);
      else this.nib(ctx, x, y, clamp(a.r * 0.12, 1.6, 3.2), ink, glow, fade);
    });
    for (const p of nibs) {
      const x = panX + p.x * zoom, y = panY + p.y * zoom;
      if (inView(x, y, 6)) this.nib(ctx, x, y, clamp(zoom * 9, 1.4, 2.8), ink, glow, 0.9);
    }
  }
  ribbon(path, channel, fraction, half, { panX, panY, zoom }) {
    const samples = channel.samples, stop = fraction * channel.length;
    const points = [];
    for (const s of samples) { if (s.distance > stop) break; points.push(s); }
    const end = pointOnChannel(channel, fraction);
    points.push(end);
    if (points.length < 2) return;
    const left = [], right = [];
    for (let i = 0; i < points.length; i++) {
      const a = points[Math.max(0, i - 1)], b = points[Math.min(points.length - 1, i + 1)];
      let dx = b.x - a.x, dy = b.y - a.y; const length = Math.hypot(dx, dy) || 1; dx /= length; dy /= length;
      const x = panX + points[i].x * zoom, y = panY + points[i].y * zoom;
      left.push([x - dy * half, y + dx * half]); right.push([x + dy * half, y - dx * half]);
    }
    path.moveTo(left[0][0], left[0][1]);
    for (const [x, y] of left) path.lineTo(x, y);
    for (let i = right.length - 1; i >= 0; i--) path.lineTo(right[i][0], right[i][1]);
    path.closePath();
  }
  bleed(ctx, x, y, r, light, alpha) {
    // A soft pool of ink where the nib touches the page.
    const pool = ctx.createRadialGradient(x, y, 0, x, y, r * 2.4);
    pool.addColorStop(0, light ? `rgba(60,36,18,${0.32 * alpha})` : `rgba(255,226,160,${0.4 * alpha})`);
    pool.addColorStop(1, light ? 'rgba(60,36,18,0)' : 'rgba(255,226,160,0)');
    ctx.fillStyle = pool; ctx.beginPath(); ctx.arc(x, y, r * 2.4, 0, TAU); ctx.fill();
  }
  nib(ctx, x, y, r, ink, glow, alpha) {
    // A small steel nib, lit only faintly so many at once read as ink, not sparks.
    ctx.save(); ctx.globalAlpha = alpha * 0.8; ctx.fillStyle = ink; ctx.shadowColor = glow; ctx.shadowBlur = r * 1.5;
    ctx.beginPath(); ctx.moveTo(x, y - r * 1.6); ctx.lineTo(x + r * 0.55, y); ctx.lineTo(x, y + r * 1.6); ctx.lineTo(x - r * 0.55, y); ctx.closePath(); ctx.fill();
    ctx.restore();
  }
  quillGlyph(ctx, x, y, size, ink, glow, alpha, time, light) {
    // A goose-feather quill held by a right hand: the nib at the tip, the vane up and to the right.
    ctx.save(); ctx.translate(x, y); ctx.rotate(0.62 + Math.sin(time / 260) * 0.06); ctx.globalAlpha = alpha;
    const s = size;
    ctx.shadowColor = light ? 'rgba(40,25,10,.28)' : glow; ctx.shadowBlur = light ? 6 : 10; ctx.shadowOffsetX = light ? 3 : 0; ctx.shadowOffsetY = light ? 4 : 0;
    // Vane.
    ctx.beginPath();
    ctx.moveTo(0, -s * 0.26);
    ctx.bezierCurveTo(-s * 0.2, -s * 0.42, -s * 0.2, -s * 0.8, -s * 0.03, -s * 1.08);
    ctx.bezierCurveTo(s * 0.05, -s * 0.9, s * 0.15, -s * 0.62, s * 0.06, -s * 0.36);
    ctx.closePath();
    ctx.fillStyle = light ? 'rgba(250,244,230,.96)' : 'rgba(240,232,212,.9)'; ctx.fill();
    ctx.shadowColor = 'transparent';
    ctx.lineWidth = Math.max(0.6, s * 0.014); ctx.strokeStyle = light ? 'rgba(59,42,28,.75)' : 'rgba(120,96,58,.75)'; ctx.stroke();
    // Barbs.
    ctx.lineWidth = Math.max(0.4, s * 0.008);
    for (let i = 0; i < 7; i++) {
      const v = -s * (0.36 + i * 0.1);
      ctx.beginPath(); ctx.moveTo(0, v); ctx.lineTo(-s * (0.09 + 0.03 * Math.sin(i)), v - s * 0.07); ctx.stroke();
      ctx.beginPath(); ctx.moveTo(0, v + s * 0.02); ctx.lineTo(s * 0.06, v - s * 0.04); ctx.stroke();
    }
    // Shaft and nib.
    ctx.strokeStyle = light ? '#3b2a1c' : '#d9c48f'; ctx.lineWidth = Math.max(0.8, s * 0.022); ctx.lineCap = 'round';
    ctx.beginPath(); ctx.moveTo(0, -s * 0.1); ctx.lineTo(-s * 0.02, -s * 1.02); ctx.stroke();
    ctx.fillStyle = ink === '#3b2a1c' ? '#2a1d12' : '#e8c77c';
    ctx.beginPath(); ctx.moveTo(0, 0); ctx.lineTo(-s * 0.035, -s * 0.13); ctx.lineTo(s * 0.035, -s * 0.13); ctx.closePath(); ctx.fill();
    ctx.restore();
  }
}
