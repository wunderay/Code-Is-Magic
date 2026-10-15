// Live display effects never enter an exported source capsule.
// Geometry and animation phases come from the diagram; time only advances playback.
import { sampleCurve, pointOnChannel, createFlow, activeEvent } from './flow.js';
export class LivingInk {
  constructor() {
    this.base=document.createElement('canvas');this.halo=document.createElement('canvas');
    this.baseContext=this.base.getContext('2d');this.haloContext=this.halo.getContext('2d');
    this.cache='';this.paths=[];this.colors=new Map();this.ink=null;this.background='#0c1112';
  }
  async prepare(svg,diagram) {
    const doc=new DOMParser().parseFromString(svg,'image/svg+xml');this.paths=[];
    for(const entity of doc.querySelectorAll('[id^=entity-]'))this.colors.set(Number(entity.id.slice(7)),entity.querySelector('[stroke]')?.getAttribute('stroke')||'#d8be87');
    this.background=doc.querySelector('rect')?.getAttribute('fill')||'#0c1112';
    const first=doc.documentElement.querySelector(':scope > rect');first?.remove();
    for(const group of doc.querySelectorAll('[data-channel]')) {
      if(group.getAttribute('data-channel')==='calls') {
        const values=group.querySelector('path')?.getAttribute('d')?.match(/-?\d+(?:\.\d+)?/g)?.map(Number);
        if(values?.length===8&&values.every(Number.isFinite))this.paths.push({from:Number(group.getAttribute('data-from')),to:Number(group.getAttribute('data-to')),...sampleCurve(values)});
      }
      group.remove();
    }
    for(const node of doc.querySelectorAll('[fill]'))if(node.getAttribute('fill')===this.background)node.setAttribute('fill','none');
    const url=URL.createObjectURL(new Blob([new XMLSerializer().serializeToString(doc)],{type:'image/svg+xml'}));
    const image=new Image();
    try {await new Promise((resolve,reject)=>{image.onload=resolve;image.onerror=()=>reject(new Error('Could not render luminous ink'));image.src=url;});}
    finally {URL.revokeObjectURL(url);}
    this.ink=image;this.cache='';
    this.seals=new Map(diagram.placements.map(p=>[p.id,p]));
    this.flow=createFlow(diagram.analysis,this.paths,diagram.placements);
  }
  paint(ctx,image,diagram,view,time,{glow,motion}) {
    const {width,height,dpr,panX,panY,zoom}=view;
    if(Math.round(width*dpr)<1||Math.round(height*dpr)<1)return;
    const key=[width,height,dpr,panX,panY,zoom].join(':');
    if(this.cache!==key) {
      for(const layer of [this.base,this.halo]) {layer.width=Math.round(width*dpr);layer.height=Math.round(height*dpr);}
      const base=this.baseContext,halo=this.haloContext;
      base.setTransform(dpr,0,0,dpr,0,0);base.fillStyle=this.background;base.fillRect(0,0,width,height);
      base.drawImage(image,panX,panY,diagram.width*zoom,diagram.height*zoom);
      halo.setTransform(dpr,0,0,dpr,0,0);halo.filter='blur(3px)';
      if(this.ink)halo.drawImage(this.ink,panX,panY,diagram.width*zoom,diagram.height*zoom);
      halo.filter='none';
      for(const seal of diagram.placements){
        const x=panX+seal.x*zoom,y=panY+seal.y*zoom,r=seal.radius*zoom;
        if(r<5||x+r*1.4<0||x-r*1.4>width||y+r*1.4<0||y-r*1.4>height)continue;
        const hex=this.colors.get(seal.id)||'#d8be87';
        const rgb=/^#[0-9a-f]{6}$/i.test(hex)?[1,3,5].map(start=>parseInt(hex.slice(start,start+2),16)).join(','):'216,190,135';
        const aura=halo.createRadialGradient(x,y,r*.65,x,y,r*1.40);
        aura.addColorStop(0,`rgba(${rgb},0)`);aura.addColorStop(.46,`rgba(${rgb},.22)`);aura.addColorStop(1,`rgba(${rgb},0)`);
        halo.fillStyle=aura;halo.beginPath();halo.arc(x,y,r*1.4,0,Math.PI*2);halo.fill();
      }
      this.cache=key;
    }
    ctx.drawImage(this.base,0,0,width,height);
    const light=this.background==='#f0e8d7';
    if(glow) {
      ctx.save();ctx.globalCompositeOperation=light?'multiply':'screen';
      ctx.globalAlpha=light ? .30 : .48;
      ctx.drawImage(this.halo,0,0,width,height);ctx.restore();
    }
    if(!motion)return;
    ctx.save();ctx.globalCompositeOperation=light?'multiply':'screen';
    // A caller kindles its own runes, sends an invocation, then receives its return.
    for(let lane=0;lane<this.flow.length;lane++) {
      const event=activeEvent(this.flow[lane],time+lane*1370);if(!event)continue;
      if(event.kind==='seal')this.wakeSeal(ctx,event,view,light,glow);
      else this.traceChannel(ctx,event,view,light,glow);
    }
    ctx.restore();
  }
  wakeSeal(ctx,event,view,light,glow) {
    const seal=this.seals.get(event.id);if(!seal)return;
    const {panX,panY,zoom,width,height}=view;
    const x=panX+seal.x*zoom,y=panY+seal.y*zoom,r=seal.radius*zoom;
    if(r<2||x+r*1.5<0||x-r*1.5>width||y+r*1.5<0||y-r*1.5>height)return;
    const t=event.progress,envelope=Math.sin(Math.PI*t),color=this.colors.get(seal.id)||'#d8be87';
    ctx.save();ctx.strokeStyle=color;ctx.shadowColor=color;ctx.shadowBlur=glow?16:0;
    // Reveal the actual glyphs and geometry beneath a rotating wedge of light.
    if(glow&&this.ink) {
      ctx.save();ctx.beginPath();ctx.moveTo(x,y);
      ctx.arc(x,y,r*1.10,t*Math.PI*2-1.1,t*Math.PI*2+.16);ctx.closePath();ctx.clip();
      ctx.globalAlpha=envelope*(light?.6:1);
      ctx.drawImage(this.ink,panX,panY,this.ink.naturalWidth*zoom,this.ink.naturalHeight*zoom);ctx.restore();
    }
    for(let ring=0;ring<3;ring++) {
      const radius=r*(.34+ring*.32),phase=t*Math.PI*2*(ring%2?-1:1)+ring*2.1;
      ctx.globalAlpha=envelope*(light?.6:.85);ctx.lineWidth=Math.max(.8,Math.min(2.4,r*.012));
      ctx.beginPath();ctx.arc(x,y,radius,phase,phase+1.15);ctx.stroke();
    }
    ctx.globalAlpha=envelope*.5;ctx.lineWidth=1;
    ctx.beginPath();ctx.arc(x,y,r*(.85+t*.5),0,Math.PI*2);ctx.stroke();
    // Small four-point scintillations bloom where the inscription wakes.
    for(let i=0;i<4;i++) {
      const angle=i*Math.PI/2+t*.6,reach=r*(.65+.35*t);
      this.star(ctx,x+Math.cos(angle)*reach,y+Math.sin(angle)*reach,Math.max(1.4,Math.min(4,r*.035)),color,envelope);
    }
    ctx.restore();
  }
  traceChannel(ctx,event,view,light,glow) {
    const channel=this.paths[event.index],{panX,panY,zoom,width,height}=view;
    const color=this.colors.get(event.reverse?channel.to:channel.from)||(light?'#8c6130':'#e7d5a1');
    ctx.save();ctx.strokeStyle=color;ctx.fillStyle=color;ctx.shadowColor=color;ctx.shadowBlur=glow?14:0;
    const head=event.reverse?1-event.progress:event.progress,direction=event.reverse?-1:1;
    // The tail follows arc length, keeping the light's speed even around bends.
    for(let i=12;i>0;i--) {
      const a=head-direction*i*.018,b=head-direction*(i-1)*.018;
      if(a<0||a>1||b<0||b>1)continue;
      const p=pointOnChannel(channel,a),q=pointOnChannel(channel,b);
      ctx.globalAlpha=(1-i/13)*(light?.8:.95);ctx.lineWidth=Math.max(1.3,Math.min(3.5,zoom*3));
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
