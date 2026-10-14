// Live display effects never enter an exported source capsule.
// Geometry and animation phases come from the diagram; time only advances playback.
export class LivingInk {
  constructor() {
    this.base=document.createElement('canvas');this.halo=document.createElement('canvas');
    this.baseContext=this.base.getContext('2d');this.haloContext=this.halo.getContext('2d');
    this.cache='';this.paths=[];this.colors=new Map();this.ink=null;this.background='#0c1112';
  }
  async prepare(svg) {
    const doc=new DOMParser().parseFromString(svg,'image/svg+xml');this.paths=[];
    for(const entity of doc.querySelectorAll('[id^=entity-]'))this.colors.set(Number(entity.id.slice(7)),entity.querySelector('[stroke]')?.getAttribute('stroke')||'#d8be87');
    this.background=doc.querySelector('rect')?.getAttribute('fill')||'#0c1112';
    const first=doc.documentElement.querySelector(':scope > rect');first?.remove();
    for(const group of doc.querySelectorAll('[data-channel]')) {
      if(group.getAttribute('data-channel')==='calls') {
        const values=group.querySelector('path')?.getAttribute('d')?.match(/-?\d+(?:\.\d+)?/g)?.map(Number);
        if(values?.length===8&&this.paths.length<96)this.paths.push(values);
      }
      group.remove();
    }
    for(const node of doc.querySelectorAll('[fill]'))if(node.getAttribute('fill')===this.background)node.setAttribute('fill','none');
    const url=URL.createObjectURL(new Blob([new XMLSerializer().serializeToString(doc)],{type:'image/svg+xml'}));
    const image=new Image();
    try {await new Promise((resolve,reject)=>{image.onload=resolve;image.onerror=()=>reject(new Error('Could not render luminous ink'));image.src=url;});}
    finally {URL.revokeObjectURL(url);}
    this.ink=image;this.cache='';
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
      ctx.globalAlpha=light ? .30 : motion ? .56+.10*Math.sin(time*.0008) : .56;
      ctx.drawImage(this.halo,0,0,width,height);ctx.restore();
    }
    if(!motion)return;
    const color=light?'#8c6130':this.background==='#090d20'?'#bdeaff':'#e7d5a1';
    // Quiet orbiting filaments reinforce the circular collar without spinning text.
    if(glow)for(const seal of diagram.placements) {
      const x=panX+seal.x*zoom,y=panY+seal.y*zoom,r=seal.radius*zoom;
      if(r<9||x+r<0||x-r>width||y+r<0||y-r>height)continue;
      const phase=seal.id*2.399963229728653+time*.00014;
      ctx.save();ctx.strokeStyle=color;ctx.globalAlpha=light ? .18 : .27;
      ctx.lineWidth=Math.max(.6,Math.min(1.3,r*.006));ctx.shadowColor=color;ctx.shadowBlur=glow?8:0;
      ctx.beginPath();ctx.arc(x,y,r*1.045,phase,phase+.28);ctx.stroke();ctx.restore();
    }
    // Each spark follows the actual cubic centerline of its braided channel.
    for(let i=0;i<this.paths.length;i++) {
      const p=this.paths[i],t=(time*(.000055+(i%5)*.000006)+i*.38196601125)%1,u=1-t;
      const x=panX+(u*u*u*p[0]+3*u*u*t*p[2]+3*u*t*t*p[4]+t*t*t*p[6])*zoom;
      const y=panY+(u*u*u*p[1]+3*u*u*t*p[3]+3*u*t*t*p[5]+t*t*t*p[7])*zoom;
      if(x<0||x>width||y<0||y>height)continue;
      ctx.save();ctx.fillStyle=color;ctx.globalAlpha=light ? .5 : .8;ctx.shadowColor=color;ctx.shadowBlur=glow?9:0;
      ctx.beginPath();ctx.arc(x,y,Math.max(.85,Math.min(1.6,zoom*2.2)),0,Math.PI*2);ctx.fill();ctx.restore();
    }
  }
}
