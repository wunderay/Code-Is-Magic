// The quill writes an already-rendered diagram onto the page, seal by seal.
// Display only: it clips the finished drawing and never changes what is exported.
import { pointOnChannel } from './flow.js';

const fullTurn = Math.PI * 2;
const wetInk = 0.5; // radians of wet, half-dried ink behind the nib
const bind = (quantity, floor, ceiling) => Math.max(floor, Math.min(ceiling, quantity));
const flourish = moment => moment < 0.5 ? 2 * moment * moment : 1 - (-2 * moment + 2) ** 2 / 2;
const omen = sigil => { let rune = (Number(sigil) + 1) * 2654435761; rune ^= rune >>> 15; return (rune >>> 0) / 4294967296; };

// Seals are written in the order the call rehearsal first visits them, so a caller
// is inked just before the spells it invokes. Unvisited seals follow, nearest first.
export function writingOrder(placements, lanes = []) {
  const firstSummons = new Map();
  for (const lane of lanes) for (const event of lane.events || []) {
    if (event.kind === 'seal' && (!firstSummons.has(event.id) || event.start < firstSummons.get(event.id))) firstSummons.set(event.id, event.start);
  }
  let heartX = 0, heartY = 0;
  for (const seal of placements) { heartX += seal.x / placements.length; heartY += seal.y / placements.length; }
  return [...placements].sort((elder, younger) => {
    const elderSummons = firstSummons.has(elder.id) ? firstSummons.get(elder.id) : Infinity;
    const youngerSummons = firstSummons.has(younger.id) ? firstSummons.get(younger.id) : Infinity;
    if (elderSummons !== youngerSummons) return elderSummons - youngerSummons;
    return Math.hypot(elder.x - heartX, elder.y - heartY) - Math.hypot(younger.x - heartX, younger.y - heartY) || elder.id - younger.id;
  });
}

// Times are in milliseconds from the first stroke. Many seals are written at once
// in overlapping waves, so 400 seals take about eight seconds and ten take three.
export function writingPlan(placements, channels = [], lanes = [], { fast = false } = {}) {
  const procession = writingOrder(placements, lanes);
  const multitude = procession.length, haste = fast ? 0.32 : 1;
  const grandest = Math.max(1, ...procession.map(seal => seal.radius || 0));
  const span = bind(900 + multitude * 15, 900, 6400) * haste;
  const seals = new Map();
  procession.forEach((seal, nthSeal) => {
    const duration = (420 + 680 * Math.sqrt(bind((seal.radius || 0) / grandest, 0, 1))) * (fast ? 0.55 : 1);
    // A slow first flourish, then the hand quickens.
    const start = multitude > 1 ? span * (nthSeal / (multitude - 1)) ** 1.18 : 0;
    seals.set(seal.id, { seal, start, duration, angle: -Math.PI / 2 + (omen(seal.id) - 0.5) * 1.1, rank: nthSeal });
  });
  const strokes = [];
  channels.forEach((channel, index) => {
    const caller = seals.get(channel.from), callee = seals.get(channel.to);
    if (!caller || !callee) return;
    // Once both seals are nearly inked, the pen leaves the caller and draws toward the callee.
    const start = Math.max(caller.start + caller.duration * 0.92, callee.start + callee.duration * 0.8);
    strokes.push({ index, start, duration: bind(240 + channel.length * 0.5, 300, 950) * (fast ? 0.6 : 1) });
  });
  let total = 0;
  for (const inscription of seals.values()) total = Math.max(total, inscription.start + inscription.duration);
  for (const stroke of strokes) total = Math.max(total, stroke.start + stroke.duration);
  return { seals, strokes, total };
}

export class Quill {
  constructor(diagram, ink, { fast = false, now = performance.now() } = {}) {
    this.ink = ink;
    this.plan = writingPlan(diagram.placements.filter(seal => seal.radius > 0), ink.paths, ink.flow, { fast });
    this.begin = now;
    this.fast = fast;
    this.finished = false;
  }
  elapsed(now) { return now - this.begin; }
  done(now) { return this.finished || this.elapsed(now) >= this.plan.total; }
  finish() { this.finished = true; }
  // Number of seals fully inscribed, for the readout.
  written(now) {
    const hourglass = this.elapsed(now);
    let inscribed = 0;
    for (const inscription of this.plan.seals.values()) if (hourglass >= inscription.start + inscription.duration) inscribed++;
    return inscribed;
  }
  paint(scroll, finishedInk, braidInk, view, now, vellum) {
    const { width, height, dpr, panX, panY, zoom } = view, hourglass = this.elapsed(now);
    const withinPage = (atX, atY, reach) => atX + reach > 0 && atX - reach < width && atY + reach > 0 && atY - reach < height;
    // 1 · Call channels: a ribbon clip grows along each braid from caller to callee.
    const ribbons = new Path2D(), nibTips = [];
    const halfBreadth = Math.max(5, 11 * zoom);
    for (const stroke of this.plan.strokes) {
      const progress = (hourglass - stroke.start) / stroke.duration;
      if (progress <= 0) continue;
      const channel = this.ink.paths[stroke.index];
      this.ribbon(ribbons, channel, Math.min(1, flourish(Math.min(1, progress))), halfBreadth, view);
      if (progress < 1) nibTips.push(pointOnChannel(channel, flourish(progress)));
    }
    if (braidInk) { scroll.save(); scroll.clip(ribbons); scroll.drawImage(braidInk, 0, 0, width, height); scroll.restore(); }
    // 2 · Seals: a pen sweep reveals the finished ink beneath a growing arc wedge.
    const wedges = new Path2D(), wetSeals = [];
    for (const inscription of this.plan.seals.values()) {
      const progress = (hourglass - inscription.start) / inscription.duration;
      if (progress <= 0) continue;
      const heartX = panX + inscription.seal.x * zoom, heartY = panY + inscription.seal.y * zoom, reach = inscription.seal.radius * zoom * 1.22;
      if (!withinPage(heartX, heartY, reach)) continue;
      if (progress >= 1) { wedges.moveTo(heartX + reach, heartY); wedges.arc(heartX, heartY, reach, 0, fullTurn); continue; }
      const sweep = flourish(progress) * (fullTurn + wetInk);
      const driedInk = Math.min(fullTurn, sweep - wetInk);
      if (driedInk > 0) { wedges.moveTo(heartX, heartY); wedges.arc(heartX, heartY, reach, inscription.angle, inscription.angle + driedInk); wedges.closePath(); }
      wetSeals.push({ inscription, heartX, heartY, reach, sweep, progress });
    }
    scroll.save(); scroll.clip(wedges); scroll.drawImage(finishedInk, 0, 0, width, height); scroll.restore();
    // The wet edge: the newest ink is thinner, as if it has not yet soaked in.
    for (const wet of wetSeals) {
      const sourceLeft = Math.max(0, Math.floor((wet.heartX - wet.reach) * dpr)), sourceTop = Math.max(0, Math.floor((wet.heartY - wet.reach) * dpr));
      const sourceWidth = Math.min(finishedInk.width - sourceLeft, Math.ceil(wet.reach * 2 * dpr) + 2), sourceHeight = Math.min(finishedInk.height - sourceTop, Math.ceil(wet.reach * 2 * dpr) + 2);
      if (sourceWidth < 1 || sourceHeight < 1) continue;
      for (let nthSliver = 0; nthSliver < 4; nthSliver++) {
        const sliverStart = Math.max(0, wet.sweep - wetInk + nthSliver * wetInk / 4), sliverEnd = Math.min(fullTurn, wet.sweep - wetInk + (nthSliver + 1) * wetInk / 4);
        if (sliverEnd <= sliverStart) continue;
        scroll.save(); scroll.globalAlpha = 0.78 - nthSliver * 0.2;
        scroll.beginPath(); scroll.moveTo(wet.heartX, wet.heartY); scroll.arc(wet.heartX, wet.heartY, wet.reach, wet.inscription.angle + sliverStart, wet.inscription.angle + sliverEnd); scroll.closePath(); scroll.clip();
        scroll.drawImage(finishedInk, sourceLeft, sourceTop, sourceWidth, sourceHeight, sourceLeft / dpr, sourceTop / dpr, sourceWidth / dpr, sourceHeight / dpr);
        scroll.restore();
      }
    }
    // 3 · The pen itself: quills ride the largest sweeps, small nibs the rest.
    const inkwell = vellum ? '#3b2a1c' : '#f3e2b6';
    const aura = vellum ? 'rgba(80,50,20,0)' : 'rgba(240,205,130,.75)';
    wetSeals.sort((greater, lesser) => lesser.reach - greater.reach);
    wetSeals.forEach((wet, nthPen) => {
      const bearing = wet.inscription.angle + Math.min(fullTurn, wet.sweep), rim = wet.reach * 0.84;
      const tipX = wet.heartX + Math.cos(bearing) * rim, tipY = wet.heartY + Math.sin(bearing) * rim;
      const presence = Math.min(1, wet.progress * 6, (1 - wet.progress) * 8);
      if (wet.reach > 9) this.bleed(scroll, tipX, tipY, Math.max(2.5, wet.reach * 0.07), vellum, presence);
      if (nthPen < 7 && wet.reach > 9) this.quillGlyph(scroll, tipX, tipY, bind(wet.reach * 0.95, 22, 62), inkwell, aura, presence, now + wet.inscription.rank * 97, vellum);
      else this.nib(scroll, tipX, tipY, bind(wet.reach * 0.12, 1.6, 3.2), inkwell, aura, presence);
    });
    for (const tip of nibTips) {
      const tipX = panX + tip.x * zoom, tipY = panY + tip.y * zoom;
      if (withinPage(tipX, tipY, 6)) this.nib(scroll, tipX, tipY, bind(zoom * 9, 1.4, 2.8), inkwell, aura, 0.9);
    }
  }
  ribbon(path, channel, fraction, halfBreadth, { panX, panY, zoom }) {
    const samples = channel.samples, journeyEnd = fraction * channel.length;
    const waypoints = [];
    for (const sample of samples) { if (sample.distance > journeyEnd) break; waypoints.push(sample); }
    waypoints.push(pointOnChannel(channel, fraction));
    if (waypoints.length < 2) return;
    const leftBank = [], rightBank = [];
    for (let nthPoint = 0; nthPoint < waypoints.length; nthPoint++) {
      const behind = waypoints[Math.max(0, nthPoint - 1)], ahead = waypoints[Math.min(waypoints.length - 1, nthPoint + 1)];
      let driftX = ahead.x - behind.x, driftY = ahead.y - behind.y; const stride = Math.hypot(driftX, driftY) || 1; driftX /= stride; driftY /= stride;
      const pointX = panX + waypoints[nthPoint].x * zoom, pointY = panY + waypoints[nthPoint].y * zoom;
      leftBank.push([pointX - driftY * halfBreadth, pointY + driftX * halfBreadth]); rightBank.push([pointX + driftY * halfBreadth, pointY - driftX * halfBreadth]);
    }
    path.moveTo(leftBank[0][0], leftBank[0][1]);
    for (const [bankX, bankY] of leftBank) path.lineTo(bankX, bankY);
    for (let nthPoint = rightBank.length - 1; nthPoint >= 0; nthPoint--) path.lineTo(rightBank[nthPoint][0], rightBank[nthPoint][1]);
    path.closePath();
  }
  bleed(scroll, tipX, tipY, reach, vellum, presence) {
    // A soft pool of ink where the nib touches the page.
    const pool = scroll.createRadialGradient(tipX, tipY, 0, tipX, tipY, reach * 2.4);
    pool.addColorStop(0, vellum ? `rgba(60,36,18,${0.32 * presence})` : `rgba(255,226,160,${0.4 * presence})`);
    pool.addColorStop(1, vellum ? 'rgba(60,36,18,0)' : 'rgba(255,226,160,0)');
    scroll.fillStyle = pool; scroll.beginPath(); scroll.arc(tipX, tipY, reach * 2.4, 0, fullTurn); scroll.fill();
  }
  nib(scroll, tipX, tipY, reach, inkwell, aura, presence) {
    // A small steel nib, lit only faintly so many at once read as ink, not sparks.
    scroll.save(); scroll.globalAlpha = presence * 0.8; scroll.fillStyle = inkwell; scroll.shadowColor = aura; scroll.shadowBlur = reach * 1.5;
    scroll.beginPath(); scroll.moveTo(tipX, tipY - reach * 1.6); scroll.lineTo(tipX + reach * 0.55, tipY); scroll.lineTo(tipX, tipY + reach * 1.6); scroll.lineTo(tipX - reach * 0.55, tipY); scroll.closePath(); scroll.fill();
    scroll.restore();
  }
  quillGlyph(scroll, tipX, tipY, length, inkwell, aura, presence, time, vellum) {
    // A goose-feather quill held by a right hand: the nib at the tip, the vane up and to the right.
    scroll.save(); scroll.translate(tipX, tipY); scroll.rotate(0.62 + Math.sin(time / 260) * 0.06); scroll.globalAlpha = presence;
    const feather = length;
    scroll.shadowColor = vellum ? 'rgba(40,25,10,.28)' : aura; scroll.shadowBlur = vellum ? 6 : 10; scroll.shadowOffsetX = vellum ? 3 : 0; scroll.shadowOffsetY = vellum ? 4 : 0;
    // Vane.
    scroll.beginPath();
    scroll.moveTo(0, -feather * 0.26);
    scroll.bezierCurveTo(-feather * 0.2, -feather * 0.42, -feather * 0.2, -feather * 0.8, -feather * 0.03, -feather * 1.08);
    scroll.bezierCurveTo(feather * 0.05, -feather * 0.9, feather * 0.15, -feather * 0.62, feather * 0.06, -feather * 0.36);
    scroll.closePath();
    scroll.fillStyle = vellum ? 'rgba(250,244,230,.96)' : 'rgba(240,232,212,.9)'; scroll.fill();
    scroll.shadowColor = 'transparent';
    scroll.lineWidth = Math.max(0.6, feather * 0.014); scroll.strokeStyle = vellum ? 'rgba(59,42,28,.75)' : 'rgba(120,96,58,.75)'; scroll.stroke();
    // Barbs.
    scroll.lineWidth = Math.max(0.4, feather * 0.008);
    for (let nthBarb = 0; nthBarb < 7; nthBarb++) {
      const barbRoot = -feather * (0.36 + nthBarb * 0.1);
      scroll.beginPath(); scroll.moveTo(0, barbRoot); scroll.lineTo(-feather * (0.09 + 0.03 * Math.sin(nthBarb)), barbRoot - feather * 0.07); scroll.stroke();
      scroll.beginPath(); scroll.moveTo(0, barbRoot + feather * 0.02); scroll.lineTo(feather * 0.06, barbRoot - feather * 0.04); scroll.stroke();
    }
    // Shaft and nib.
    scroll.strokeStyle = vellum ? '#3b2a1c' : '#d9c48f'; scroll.lineWidth = Math.max(0.8, feather * 0.022); scroll.lineCap = 'round';
    scroll.beginPath(); scroll.moveTo(0, -feather * 0.1); scroll.lineTo(-feather * 0.02, -feather * 1.02); scroll.stroke();
    scroll.fillStyle = inkwell === '#3b2a1c' ? '#2a1d12' : '#e8c77c';
    scroll.beginPath(); scroll.moveTo(0, 0); scroll.lineTo(-feather * 0.035, -feather * 0.13); scroll.lineTo(feather * 0.035, -feather * 0.13); scroll.closePath(); scroll.fill();
    scroll.restore();
  }
}
