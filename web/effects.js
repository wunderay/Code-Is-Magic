// Live display effects never enter an exported source capsule.
// Geometry and animation phases come from the diagram; time only advances playback.
import { sampleCurve, pointOnChannel, createFlow, activeEvent, illumination } from './flow.js';
// Stroke the current path (or the given one) with a soft glow of about `blur` pixels: two wide,
// faint passes under the line. Canvas shadows look alike but blur every stroke anew, every frame.
function halo(ctx,width,blur,path) {
  const stroke=()=>path?ctx.stroke(path):ctx.stroke(),alpha=ctx.globalAlpha;
  if(blur>0){ctx.globalAlpha=alpha*.14;ctx.lineWidth=width+blur*1.1;stroke();ctx.globalAlpha=alpha*.28;ctx.lineWidth=width+blur*.45;stroke();ctx.globalAlpha=alpha;}
  ctx.lineWidth=width;stroke();
}
// Pixels per cached bitmap: about 32 MB each, enough for twice the fitted zoom on a typical screen.
const rasterArea=8e6;
export class LivingInk {
  constructor() {
    this.base=document.createElement('canvas');this.halo=document.createElement('canvas');
    this.baseContext=this.base.getContext('2d');this.haloContext=this.halo.getContext('2d');
    this.cache='';this.braidCache='';this.paths=[];this.braidSheet=document.createElement('canvas');this.braidPen=this.braidSheet.getContext('2d');this.colors=new Map();this.ink=null;this.background='#0c1112';
  }
  async prepare(svg,diagram) {
    const doc=new DOMParser().parseFromString(svg,'image/svg+xml'),root=doc.documentElement;this.paths=[];
    for(const entity of doc.querySelectorAll('[id^=entity-]'))this.colors.set(Number(entity.id.slice(7)),entity.querySelector('[stroke]')?.getAttribute('stroke')||'#d8be87');
    this.background=doc.querySelector('rect')?.getAttribute('fill')||'#0c1112';
    const first=root.querySelector(':scope > rect');first?.remove();
    // The drawing comes apart in its own painting order: starfield and frame, then every call braid,
    // then each seal on its dark disc. Painting those layers back in turn rebuilds the whole drawing,
    // so the full SVG never has to be decoded a second time beside its parts.
    const layer=()=>root.cloneNode(false),decor=layer(),braids=layer();
    for(const grimoireDefs of root.querySelectorAll(':scope > defs'))braids.append(grimoireDefs.cloneNode(true));
    for(const node of [...root.children]) {
      if(node.localName==='defs'||node.hasAttribute('data-channel')||node.id?.startsWith('entity-'))continue;
      decor.append(node);
    }
    for(const braid of root.querySelectorAll('[data-channel]'))braids.append(braid.cloneNode(true));
    for(const group of root.querySelectorAll('[data-channel]')) {
      if(group.getAttribute('data-channel')==='calls') {
        const values=group.querySelector('path')?.getAttribute('d')?.match(/-?\d+(?:\.\d+)?/g)?.map(Number);
        if(values?.length===8&&values.every(Number.isFinite))this.paths.push({from:Number(group.getAttribute('data-from')),to:Number(group.getAttribute('data-to')),...sampleCurve(values),strands:[...group.querySelectorAll(':scope > path')].slice(-2).map(path=>new Path2D(path.getAttribute('d')))});
      }
      group.remove();
    }
    // Each seal's disc hides the braids beneath it. The live ink leaves it out so the glow shows through.
    this.discs=[];
    for(const entity of root.querySelectorAll(':scope > [id^=entity-]')) {
      const disc=entity.firstElementChild;
      if(disc?.localName==='circle'&&disc.getAttribute('fill')===this.background&&!disc.hasAttribute('stroke')) {
        const [x,y,r]=['cx','cy','r'].map(name=>Number(disc.getAttribute(name)));
        if([x,y,r].every(Number.isFinite)){this.discs.push({x,y,r});disc.remove();}
      }
    }
    for(const node of doc.querySelectorAll('[fill]'))if(node.getAttribute('fill')===this.background)node.setAttribute('fill','none');
    const load=(element,message)=>{
      const url=URL.createObjectURL(new Blob([new XMLSerializer().serializeToString(element)],{type:'image/svg+xml'})),image=new Image();
      return new Promise((resolve,reject)=>{image.onload=()=>resolve(image);image.onerror=()=>reject(new Error(message));image.src=url;}).finally(()=>URL.revokeObjectURL(url));
    };
    const message='The inscription image could not render.';
    [this.decor,this.braids,this.ink]=await Promise.all([decor.children.length?load(decor,message):null,load(braids,message),load(root,message)]);
    this.cache='';
    this.names=new Map(diagram.analysis.entities.map(e=>[e.id,e.qualified_name]));
    this.seals=new Map(diagram.placements.map(p=>[p.id,p]));
    this.flow=createFlow(diagram.analysis,this.paths,diagram.placements);
    const bounds={left:Infinity,top:Infinity,right:-Infinity,bottom:-Infinity};
    const include=(x,y,r=0)=>{bounds.left=Math.min(bounds.left,x-r);bounds.top=Math.min(bounds.top,y-r);bounds.right=Math.max(bounds.right,x+r);bounds.bottom=Math.max(bounds.bottom,y+r);};
    for(const seal of diagram.placements)include(seal.x,seal.y,seal.radius*1.25);
    for(const path of this.paths)for(const point of path.samples)include(point.x,point.y,8);
    this.bounds=Number.isFinite(bounds.left)?bounds:{left:0,top:0,right:diagram.width,bottom:diagram.height};
  }
  // Rasterize each SVG once, in diagram space, at a resolution near what the view needs.
  // Redrawing an 11 MB SVG on every pan or zoom step stalled the page for hundreds of milliseconds
  // per frame; blitting these bitmaps costs a few. The SVG itself is drawn again only to sharpen a
  // still view that is zoomed in past the bitmaps' resolution.
  rasterize(diagram,{zoom,dpr}) {
    const fit=Math.sqrt(rasterArea/Math.max(1,diagram.width*diagram.height));
    const scale=this.rasterScale=Math.max(.02,Math.min(1,fit,zoom*dpr*2));
    const sheet=paint=>{
      const canvas=document.createElement('canvas');canvas.width=Math.max(1,Math.round(diagram.width*scale));canvas.height=Math.max(1,Math.round(diagram.height*scale));
      const pen=canvas.getContext('2d');pen.setTransform(scale,0,0,scale,0,0);paint(pen);
      return canvas;
    };
    // Each vector layer is rasterized once; the whole drawing is then assembled from those bitmaps.
    this.inkRaster=sheet(pen=>pen.drawImage(this.ink,0,0,diagram.width,diagram.height));
    this.braidRaster=sheet(pen=>pen.drawImage(this.braids,0,0,diagram.width,diagram.height));
    this.baseRaster=sheet(pen=>this.drawWhole(pen,diagram,{braids:this.braidRaster,ink:this.inkRaster}));
    // The halo is soft, so a quarter of the resolution is plenty.
    const halo=document.createElement('canvas'),haloScale=scale/4;
    halo.width=Math.max(1,Math.round(diagram.width*haloScale));halo.height=Math.max(1,Math.round(diagram.height*haloScale));
    const glow=halo.getContext('2d');
    // The same blur the screen used: 3 CSS pixels at the zoom this bitmap was made for.
    glow.filter=`blur(${Math.max(.5,3*haloScale/zoom).toFixed(2)}px)`;
    if(this.inkRaster&&glow.filter!=='none')glow.drawImage(this.inkRaster,0,0,halo.width,halo.height);
    glow.filter='none';glow.setTransform(haloScale,0,0,haloScale,0,0);
    for(const seal of diagram.placements){
      if(seal.radius*zoom<5)continue;
      const hex=this.colors.get(seal.id)||'#d8be87';
      const rgb=/^#[0-9a-f]{6}$/i.test(hex)?[1,3,5].map(start=>parseInt(hex.slice(start,start+2),16)).join(','):'216,190,135';
      const aura=glow.createRadialGradient(seal.x,seal.y,seal.radius*.65,seal.x,seal.y,seal.radius*1.40);
      aura.addColorStop(0,`rgba(${rgb},0)`);aura.addColorStop(.46,`rgba(${rgb},.22)`);aura.addColorStop(1,`rgba(${rgb},0)`);
      glow.fillStyle=aura;glow.beginPath();glow.arc(seal.x,seal.y,seal.radius*1.4,0,Math.PI*2);glow.fill();
    }
    this.haloRaster=halo;
    // A lit seal shows its blurred halo and its ink twice over, blended onto the page. Those three
    // layers are blended here once, onto black (or white on vellum), which leaves the page unchanged
    // under screen (or multiply); each frame then draws one bitmap per lit seal instead of three.
    const light=this.background==='#f0e8d7';
    this.relightRaster=sheet(pen=>{
      pen.setTransform(1,0,0,1,0,0);
      const {width,height}=pen.canvas;
      pen.fillStyle=light?'#fff':'#000';pen.fillRect(0,0,width,height);
      pen.globalCompositeOperation=light?'multiply':'screen';
      pen.globalAlpha=light?.6:.9;pen.drawImage(this.haloRaster,0,0,width,height);
      pen.globalAlpha=light?.75:1;pen.drawImage(this.inkRaster,0,0,width,height);pen.drawImage(this.inkRaster,0,0,width,height);
    });
    // The plain ink bitmap is not kept: without glow the lit seals need ink without the halo, which
    // glowless() makes from the vector ink only if glow is turned off.
    this.inkRaster=null;this.glowlessRaster=null;
  }
  glowless() {
    if(this.glowlessRaster)return this.glowlessRaster;
    const light=this.background==='#f0e8d7',canvas=document.createElement('canvas');
    canvas.width=this.relightRaster.width;canvas.height=this.relightRaster.height;
    const pen=canvas.getContext('2d'),{width,height}=canvas;
    pen.fillStyle=light?'#fff':'#000';pen.fillRect(0,0,width,height);
    pen.globalCompositeOperation=light?'multiply':'screen';pen.globalAlpha=light?.75:1;
    pen.drawImage(this.ink,0,0,width,height);pen.drawImage(this.ink,0,0,width,height);
    return this.glowlessRaster=canvas;
  }
  // The finished drawing, in diagram units, layer by layer in the original painting order.
  drawWhole(pen,diagram,{braids=this.braids,ink=this.ink}={}) {
    const {width,height}=diagram;
    pen.fillStyle=this.background;pen.fillRect(0,0,width,height);
    if(this.decor)pen.drawImage(this.decor,0,0,width,height);
    pen.drawImage(braids,0,0,width,height);
    pen.beginPath();for(const {x,y,r} of this.discs){pen.moveTo(x+r,y);pen.arc(x,y,r,0,Math.PI*2);}pen.fill();
    pen.drawImage(ink,0,0,width,height);
  }
  // True while the view shows a zoom the bitmaps cannot render sharply; the caller repaints once it settles.
  get blurred() { return this.cacheSoft; }
  paint(ctx,diagram,view,time,{glow,motion,writing=null,now=performance.now(),awaken=1}) {
    const {width,height,dpr,panX,panY,zoom}=view;
    if(Math.round(width*dpr)<1||Math.round(height*dpr)<1)return;
    if(!this.baseRaster)this.rasterize(diagram,view);
    const key=[width,height,dpr,panX,panY,zoom].join(':'),settled=key===this.lastKey;this.lastKey=key;
    const sharp=zoom*dpr<=this.rasterScale*1.05;
    if(this.cache!==key||(this.cacheSoft&&settled)) {
      for(const layer of [this.base,this.halo]) {const w=Math.round(width*dpr),h=Math.round(height*dpr);if(layer.width!==w||layer.height!==h){layer.width=w;layer.height=h;}}
      const base=this.baseContext;
      base.setTransform(dpr,0,0,dpr,0,0);base.fillStyle=this.background;base.fillRect(0,0,width,height);
      // A moving view uses the bitmap; a still, deeply zoomed one is drawn from the vector original.
      if(sharp||!settled)base.drawImage(this.baseRaster,panX,panY,diagram.width*zoom,diagram.height*zoom);
      else {base.save();base.translate(panX,panY);base.scale(zoom,zoom);this.drawWhole(base,diagram);base.restore();}
      this.cache=key;this.cacheSoft=!sharp&&!settled;this.baseVersion=(this.baseVersion||0)+1;
    }
    const light=this.background==='#f0e8d7';
    if(writing&&!writing.done(now)) {
      if(this.braidRaster&&this.braidCache!==key) {
        const sheet=this.braidSheet,pen=this.braidPen,w=Math.round(width*dpr),h=Math.round(height*dpr);
        if(sheet.width!==w||sheet.height!==h){sheet.width=w;sheet.height=h;}else pen.clearRect(0,0,w,h);
        pen.setTransform(dpr,0,0,dpr,0,0);pen.drawImage(this.braidRaster,panX,panY,diagram.width*zoom,diagram.height*zoom);this.braidCache=key;
      }
      ctx.fillStyle=this.background;ctx.fillRect(0,0,width,height);
      writing.paint(ctx,this.base,this.braidRaster?this.braidSheet:null,view,now,light);
      return;
    }
    // The braids are only needed while the quill writes; release their memory afterwards.
    if(this.braidRaster){this.braidRaster=null;this.braidSheet.width=this.braidSheet.height=0;this.braidCache='';}
    if(glow&&awaken>=1) {
      // The settled glow is blended into its own copy of the page once per view, not on every frame.
      if(this.litVersion!==this.baseVersion) {
        const lit=this.haloContext;
        lit.setTransform(1,0,0,1,0,0);lit.globalCompositeOperation='copy';lit.drawImage(this.base,0,0);
        this.glowOver(lit,diagram,view,1,light);this.litVersion=this.baseVersion;
      }
      ctx.drawImage(this.halo,0,0,width,height);
    } else {
      ctx.drawImage(this.base,0,0,width,height);
      if(glow)this.glowOver(ctx,diagram,view,awaken,light);
    }
    if(!motion)return;
    ctx.save();ctx.globalCompositeOperation=light?'multiply':'screen';
    // A caller kindles its own runes, sends an invocation, then receives its return.
    for(let lane=0;lane<this.flow.length;lane++) {
      const event=activeEvent(this.flow[lane],time+lane*1370);if(!event)continue;
      for(const seal of illumination(event,this.paths))if(seal.intensity>0)this.lightSeal(ctx,seal.id,seal.intensity,view,light,glow);
      if(event.kind==='seal')this.wakeSeal(ctx,event,view,light,glow);
      else this.traceChannel(ctx,event,view,light,glow);
    }
    ctx.restore();
  }
  glowOver(ctx,diagram,{panX,panY,zoom,dpr},awaken,light) {
    ctx.save();ctx.setTransform(dpr,0,0,dpr,0,0);ctx.globalCompositeOperation=light?'multiply':'screen';
    ctx.globalAlpha=(light ? .18 : .20)*awaken;
    ctx.drawImage(this.haloRaster,panX,panY,diagram.width*zoom,diagram.height*zoom);ctx.restore();
  }
  describe(time) {
    for(let lane=0;lane<this.flow.length;lane++) {
      const event=activeEvent(this.flow[lane],time+lane*1370);if(!event)continue;
      if(event.kind==='seal')return `${this.names.get(event.id)||'A spell'} awakens`;
      const path=this.paths[event.index],from=this.names.get(path.from)||'Caller',to=this.names.get(path.to)||'callee';
      return event.reverse?`${to} returns to ${from}`:`${from} calls ${to}`;
    }
    return 'The grimoire rests';
  }
  lightSeal(ctx,id,intensity,view,light,glow) {
    const seal=this.seals.get(id);if(!seal)return;
    const {panX,panY,zoom,width,height}=view;
    const x=panX+seal.x*zoom,y=panY+seal.y*zoom,r=seal.radius*zoom;
    if(r<1||x+r*1.8<0||x-r*1.8>width||y+r*1.8<0||y-r*1.8>height)return;
    const color=this.colors.get(id)||'#d8be87';
    ctx.save();ctx.globalAlpha=intensity;
    if(glow) {
      const aura=ctx.createRadialGradient(x,y,r*.35,x,y,r*1.75);
      aura.addColorStop(0,`${color}0a`);aura.addColorStop(.37,`${color}24`);
      aura.addColorStop(.52,`${color}80`);aura.addColorStop(.7,`${color}27`);aura.addColorStop(1,`${color}00`);
      ctx.fillStyle=aura;ctx.beginPath();ctx.arc(x,y,r*1.75,0,Math.PI*2);ctx.fill();
    }
    // Relight the entire original seal: rings, geometry, glyphs, and name contour, with its blurred halo
    // (the ink already blurred, standing in for a canvas shadow that was recomputed every frame).
    // Filled as a pattern rather than clipped: a clip costs a whole extra layer on every frame.
    const ink=this.inkPattern(ctx,glow?this.relightRaster:this.glowless(),view);
    if(ink){ctx.globalAlpha=intensity;ctx.fillStyle=ink;ctx.beginPath();ctx.arc(x,y,r*1.1,0,Math.PI*2);ctx.fill();}
    ctx.strokeStyle=color;ctx.beginPath();ctx.arc(x,y,r*1.025,0,Math.PI*2);
    halo(ctx,Math.max(1.2,Math.min(2.4,r*.018)),glow?16:0);ctx.restore();
  }
  wakeSeal(ctx,event,view,light,glow) {
    const seal=this.seals.get(event.id);if(!seal)return;
    const {panX,panY,zoom,width,height}=view;
    const x=panX+seal.x*zoom,y=panY+seal.y*zoom,r=seal.radius*zoom;
    if(r<2||x+r*1.5<0||x-r*1.5>width||y+r*1.5<0||y-r*1.5>height)return;
    const t=event.progress,envelope=Math.sin(Math.PI*t),color=this.colors.get(seal.id)||'#d8be87';
    ctx.save();ctx.strokeStyle=color;
    // Reveal the actual glyphs and geometry beneath a rotating wedge of light.
    const ink=glow&&this.inkPattern(ctx,this.relightRaster,view);
    if(ink) {
      // The bitmap holds the ink twice over and its halo; this alpha keeps the wedge about as bright as one pass of ink.
      ctx.save();ctx.globalAlpha=envelope*(light?.45:.6);ctx.fillStyle=ink;
      ctx.beginPath();ctx.moveTo(x,y);ctx.arc(x,y,r*1.10,t*Math.PI*2-1.1,t*Math.PI*2+.16);ctx.closePath();ctx.fill();ctx.restore();
    }
    for(let ring=0;ring<3;ring++) {
      const radius=r*(.34+ring*.32),phase=t*Math.PI*2*(ring%2?-1:1)+ring*2.1;
      ctx.globalAlpha=envelope*(light?.6:.85);
      ctx.beginPath();ctx.arc(x,y,radius,phase,phase+1.15);halo(ctx,Math.max(.8,Math.min(2.4,r*.012)),glow?16:0);
    }
    ctx.globalAlpha=envelope*.5;
    ctx.beginPath();ctx.arc(x,y,r*(.85+t*.5),0,Math.PI*2);halo(ctx,1,glow?16:0);
    // Small four-point scintillations bloom where the inscription wakes.
    for(let i=0;i<4;i++) {
      const angle=i*Math.PI/2+t*.6,reach=r*(.65+.35*t);
      this.star(ctx,x+Math.cos(angle)*reach,y+Math.sin(angle)*reach,Math.max(1.4,Math.min(4,r*.035)),color,envelope);
    }
    ctx.restore();
  }
  // A diagram-space bitmap as a fill, placed under the current view.
  inkPattern(ctx,bitmap,{panX,panY,zoom}) {
    if(!bitmap)return null;
    let pattern=this.patterns?.get(bitmap);
    if(!pattern){pattern=ctx.createPattern(bitmap,'no-repeat');(this.patterns??=new WeakMap()).set(bitmap,pattern);}
    const scale=zoom/this.rasterScale;
    pattern.setTransform(new DOMMatrix([scale,0,0,scale,panX,panY]));
    return pattern;
  }
  traceChannel(ctx,event,view,light,glow) {
    const channel=this.paths[event.index],{panX,panY,zoom,width,height}=view;
    const color=this.colors.get(event.reverse?channel.to:channel.from)||(light?'#8c6130':'#e7d5a1');
    ctx.save();ctx.strokeStyle=color;ctx.fillStyle=color;
    const head=event.reverse?1-event.progress:event.progress,direction=event.reverse?-1:1;
    // The actual braided invocation glows while it is called, in the caller's hue.
    if(glow) {
      ctx.save();ctx.translate(panX,panY);ctx.scale(zoom,zoom);
      ctx.globalAlpha=.18+.35*Math.sin(Math.PI*event.progress);
      for(const strand of channel.strands)halo(ctx,Math.max(1,1/zoom),12/zoom,strand);
      ctx.restore();
    }
    // A luminous front fills the travelled channel, rather than an isolated dot.
    const segment=[];
    for(let i=0;i<=40;i++)segment.push(pointOnChannel(channel,event.reverse?1-event.progress*i/40:event.progress*i/40));
    // These three widths already make the glow; a canvas shadow on top only cost time.
    for(const [lineWidth,alpha] of [[9,.12],[4,.38],[1.5,.95]]) {
      ctx.globalAlpha=alpha;ctx.lineWidth=lineWidth;
      ctx.beginPath();segment.forEach((p,i)=>i?ctx.lineTo(panX+p.x*zoom,panY+p.y*zoom):ctx.moveTo(panX+p.x*zoom,panY+p.y*zoom));ctx.stroke();
    }
    for(let i=10;i>0;i--) {
      const a=head-direction*i*.016,b=head-direction*(i-1)*.016;
      if(a<0||a>1||b<0||b>1)continue;
      const p=pointOnChannel(channel,a),q=pointOnChannel(channel,b);
      ctx.globalAlpha=(1-i/11)*.95;ctx.lineWidth=3;
      ctx.beginPath();ctx.moveTo(panX+p.x*zoom,panY+p.y*zoom);ctx.lineTo(panX+q.x*zoom,panY+q.y*zoom);ctx.stroke();
    }
    const p=pointOnChannel(channel,head),x=panX+p.x*zoom,y=panY+p.y*zoom;
    if(x>-20&&x<width+20&&y>-20&&y<height+20) {
      this.star(ctx,x,y,Math.max(2.5,Math.min(5,zoom*4)),color,light?.85:1);
      if(!light){ctx.globalAlpha=.95;ctx.fillStyle='#fff7df';ctx.beginPath();ctx.arc(x,y,1.2,0,Math.PI*2);ctx.fill();}
    }
    ctx.restore();
  }
  star(ctx,x,y,r,color,alpha) {
    ctx.globalAlpha=alpha;ctx.fillStyle=color;ctx.beginPath();
    ctx.moveTo(x,y-r);ctx.lineTo(x+r*.25,y-r*.25);ctx.lineTo(x+r,y);ctx.lineTo(x+r*.25,y+r*.25);
    ctx.lineTo(x,y+r);ctx.lineTo(x-r*.25,y+r*.25);ctx.lineTo(x-r,y);ctx.lineTo(x-r*.25,y-r*.25);ctx.closePath();ctx.fill();
  }
}
