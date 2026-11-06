// Decorative page atmosphere: drifting motes behind the page and a sparkle
// burst on the download button. Independent of app.js; it only listens to DOM
// events and never intercepts pointer input.
const reduced = matchMedia('(prefers-reduced-motion: reduce)');
const root = document.documentElement;

function themeColors() {
  const style = getComputedStyle(root);
  const light = root.dataset.theme === 'light';
  return {
    gold: style.getPropertyValue('--gold').trim() || '#d8bd88',
    star: style.getPropertyValue('--star').trim() || '#b9b7cf',
    strength: light ? 0.5 : 1,
  };
}

function motes() {
  const canvas = document.createElement('canvas');
  canvas.className = 'enchant-sky';
  canvas.setAttribute('aria-hidden', 'true');
  document.body.prepend(canvas);
  const ctx = canvas.getContext('2d', { alpha: true });
  if (!ctx) return;
  let width = 0, height = 0, ratio = 1, particles = [], colors = themeColors(), frame = 0, last = 0;

  const seed = () => {
    const count = Math.round(Math.min(110, Math.max(36, (width * height) / 14000)));
    particles = Array.from({ length: count }, (_, index) => {
      const mote = index % 4 === 0;
      return {
        x: Math.random() * width,
        y: Math.random() * height,
        r: mote ? 0.9 + Math.random() * 1.3 : 0.35 + Math.random() * 0.75,
        vx: (Math.random() - 0.5) * 0.006,
        vy: mote ? -(0.004 + Math.random() * 0.01) : -(Math.random() * 0.002),
        phase: Math.random() * Math.PI * 2,
        speed: 0.0004 + Math.random() * 0.0012,
        mote,
      };
    });
  };

  const resize = () => {
    ratio = Math.min(devicePixelRatio || 1, 1.5);
    width = innerWidth; height = innerHeight;
    canvas.width = Math.round(width * ratio);
    canvas.height = Math.round(height * ratio);
    ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
    seed();
  };

  const draw = (time, dt) => {
    ctx.clearRect(0, 0, width, height);
    for (const p of particles) {
      if (dt) {
        p.x += p.vx * dt; p.y += p.vy * dt;
        if (p.y < -6) { p.y = height + 6; p.x = Math.random() * width; }
        if (p.x < -6) p.x = width + 6; else if (p.x > width + 6) p.x = -6;
      }
      const twinkle = 0.5 + 0.5 * Math.sin(p.phase + time * p.speed);
      const alpha = (p.mote ? 0.18 + 0.42 * twinkle : 0.12 + 0.5 * twinkle) * colors.strength;
      ctx.globalAlpha = alpha;
      ctx.fillStyle = p.mote ? colors.gold : colors.star;
      if (p.mote) {
        ctx.shadowColor = colors.gold;
        ctx.shadowBlur = 6 * p.r;
      } else ctx.shadowBlur = 0;
      ctx.beginPath(); ctx.arc(p.x, p.y, p.r, 0, Math.PI * 2); ctx.fill();
    }
    ctx.globalAlpha = 1; ctx.shadowBlur = 0;
  };

  const tick = time => {
    frame = 0;
    const dt = last ? Math.min(time - last, 64) : 0;
    last = time;
    draw(time, dt);
    schedule();
  };
  const schedule = () => {
    if (!frame && !document.hidden && !reduced.matches) frame = requestAnimationFrame(tick);
  };
  const stop = () => { if (frame) cancelAnimationFrame(frame); frame = 0; last = 0; };
  const refresh = () => { stop(); if (reduced.matches) draw(0, 0); else schedule(); };

  let resizeTimer;
  addEventListener('resize', () => { clearTimeout(resizeTimer); resizeTimer = setTimeout(() => { resize(); refresh(); }, 120); }, { passive: true });
  document.addEventListener('visibilitychange', () => document.hidden ? stop() : refresh());
  reduced.addEventListener?.('change', refresh);
  new MutationObserver(() => { colors = themeColors(); if (reduced.matches || !frame) draw(0, 0); })
    .observe(root, { attributes: true, attributeFilter: ['data-theme'] });
  resize();
  refresh();
}

function sparkles() {
  const button = document.getElementById('download-magic');
  if (!button || typeof Element.prototype.animate !== 'function') return;
  const glyphs = ['✦', '✧', '·', '✶', '⋆'];
  // Capture phase: runs before app.js's onclick handler marks the button busy.
  document.addEventListener('click', event => {
    if (reduced.matches || !event.target.closest?.('#download-magic') || button.disabled) return;
    const box = button.getBoundingClientRect();
    const layer = document.createElement('div');
    layer.className = 'enchant-burst';
    layer.setAttribute('aria-hidden', 'true');
    layer.style.left = `${box.left + box.width / 2}px`;
    layer.style.top = `${box.top + box.height / 2}px`;
    document.body.append(layer);
    const count = 16;
    let remaining = count;
    for (let i = 0; i < count; i++) {
      const spark = document.createElement('span');
      spark.textContent = glyphs[i % glyphs.length];
      layer.append(spark);
      const angle = (i / count) * Math.PI * 2 + Math.random() * 0.4;
      const distance = 70 + Math.random() * 60;
      const x = Math.cos(angle) * distance, y = Math.sin(angle) * distance * 0.7 - 10;
      const scale = 0.6 + Math.random() * 0.8;
      spark.animate([
        { transform: 'translate(-50%,-50%) scale(.2)', opacity: 0, easing: 'cubic-bezier(.2,.7,.3,1)' },
        { transform: `translate(calc(-50% + ${x * 0.7}px),calc(-50% + ${y * 0.7}px)) scale(${scale})`, opacity: 1, offset: 0.3, easing: 'ease-out' },
        { transform: `translate(calc(-50% + ${x * 0.9}px),calc(-50% + ${y * 0.9 + 6}px)) scale(${scale * 0.8}) rotate(45deg)`, opacity: 0.85, offset: 0.65, easing: 'ease-in' },
        { transform: `translate(calc(-50% + ${x}px),calc(-50% + ${y + 16}px)) scale(${scale * 0.3}) rotate(90deg)`, opacity: 0 },
      ], { duration: 850 + Math.random() * 350, fill: 'forwards' })
        .finished.catch(() => {}).then(() => { if (--remaining === 0) layer.remove(); });
    }
    const ring = document.createElement('i');
    layer.append(ring);
    ring.animate([{ transform: 'translate(-50%,-50%) scale(.3)', opacity: 0.8 }, { transform: 'translate(-50%,-50%) scale(2.6)', opacity: 0 }],
      { duration: 650, easing: 'ease-out', fill: 'forwards' });
  }, true);
}

function start() { motes(); sparkles(); }
if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', start, { once: true });
else start();
