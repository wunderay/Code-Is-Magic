// A deterministic structural rehearsal, never execution of untrusted source.
// Calls are visited in lexical source order; returns retrace their invocation.
export function sampleCurve(points,steps=64) {
  const samples=[];let length=0;
  for(let i=0;i<=steps;i++) {
    const t=i/steps,u=1-t;
    const x=u*u*u*points[0]+3*u*u*t*points[2]+3*u*t*t*points[4]+t*t*t*points[6];
    const y=u*u*u*points[1]+3*u*u*t*points[3]+3*u*t*t*points[5]+t*t*t*points[7];
    if(i)length+=Math.hypot(x-samples[i-1].x,y-samples[i-1].y);
    samples.push({x,y,distance:length});
  }
  return {samples,length};
}
export function pointOnChannel(channel,fraction) {
  const samples=channel.samples,distance=Math.max(0,Math.min(1,fraction))*channel.length;
  let lo=1,hi=samples.length-1;
  while(lo<hi){const mid=(lo+hi)>>1;if(samples[mid].distance<distance)lo=mid+1;else hi=mid;}
  const a=samples[lo-1],b=samples[lo],t=(distance-a.distance)/(b.distance-a.distance||1);
  return {x:a.x+(b.x-a.x)*t,y:a.y+(b.y-a.y)*t};
}
function callOrder(entity,target) {
  const tokens=entity?.tokens||[];
  const index=tokens.findIndex((token,i)=>token===target?.name&&tokens[i+1]==='(');
  if(index<0)return Infinity;
  // Arguments finish before their enclosing call begins: outer(inner()).
  let depth=0;
  for(let i=index+1;i<tokens.length;i++){if(tokens[i]==='(')depth++;else if(tokens[i]===')'&&--depth===0)return i;}
  return index;
}
export function createFlow(analysis,channels,placements) {
  const entities=new Map(analysis.entities.map(e=>[e.id,e]));
  const seals=new Set(placements.map(p=>p.id));
  const outgoing=new Map(),incoming=new Set();
  channels.forEach((channel,index)=>{
    if(!seals.has(channel.from)||!seals.has(channel.to))return;
    const list=outgoing.get(channel.from)||[];list.push(index);outgoing.set(channel.from,list);incoming.add(channel.to);
  });
  for(const [id,list] of outgoing)list.sort((a,b)=>callOrder(entities.get(id),entities.get(channels[a].to))-callOrder(entities.get(id),entities.get(channels[b].to))||a-b);
  const entry=id=>/^(main|Main|__main__)$/.test(entities.get(id)?.name||'');
  const roots=[...seals].sort((a,b)=>Number(entry(b))-Number(entry(a))||Number(incoming.has(a))-Number(incoming.has(b))||a-b);
  const visited=new Set(),seenSeals=new Set(),tours=[];
  for(const root of roots) {
    if(seenSeals.has(root)&&!(outgoing.get(root)||[]).some(i=>!visited.has(i)))continue;
    const events=[];let duration=0;
    const add=(event,span)=>{events.push({...event,start:duration,duration:span});duration+=span;};
    const stack=[{id:root,next:0,via:null}];
    seenSeals.add(root);add({kind:'seal',id:root},760);
    while(stack.length) {
      const frame=stack[stack.length-1],list=outgoing.get(frame.id)||[];
      let index;while(frame.next<list.length){const candidate=list[frame.next++];if(!visited.has(candidate)){index=candidate;break;}}
      if(index!==undefined) {
        visited.add(index);const channel=channels[index];
        const travel=Math.max(700,Math.min(1800,channel.length*1.6));
        add({kind:'channel',index,reverse:false},travel);
        seenSeals.add(channel.to);add({kind:'seal',id:channel.to},760);
        // Each edge rehearses once, bounding recursive and mutually recursive graphs.
        stack.push({id:channel.to,next:0,via:index});
      } else {
        stack.pop();
        if(frame.via!==null) {
          const channel=channels[frame.via];
          add({kind:'channel',index:frame.via,reverse:true},Math.max(700,Math.min(1800,channel.length*1.6)));
          add({kind:'seal',id:channel.from},460);
        }
      }
    }
    tours.push({events,duration:duration+1100});
  }
  // Independent entry points may shimmer together; a lane always preserves its stack.
  const lanes=Array.from({length:Math.min(5,tours.length)},()=>({events:[],duration:0}));
  for(const tour of tours) {
    const lane=lanes.reduce((a,b)=>a.duration<=b.duration?a:b);
    for(const event of tour.events)lane.events.push({...event,start:event.start+lane.duration});
    lane.duration+=tour.duration;
  }
  return lanes;
}
export function activeEvent(lane,time) {
  if(!lane.duration)return null;
  const phase=((time%lane.duration)+lane.duration)%lane.duration;
  let lo=0,hi=lane.events.length;
  while(lo<hi){const mid=(lo+hi)>>1;if(lane.events[mid].start<=phase)lo=mid+1;else hi=mid;}
  const event=lane.events[lo-1];
  return event&&phase<event.start+event.duration?{...event,progress:(phase-event.start)/event.duration}:null;
}
// Whole seals remain lit during a transfer. The destination awakens on arrival.
export function illumination(event,channels) {
  if(event.kind==='seal')return [{id:event.id,intensity:.55+.45*Math.sin(Math.PI*event.progress)}];
  const channel=channels[event.index];
  if(!channel)return [];
  const from=event.reverse?channel.to:channel.from,to=event.reverse?channel.from:channel.to;
  const arrival=Math.max(0,(event.progress-.72)/.28);
  return [{id:from,intensity:.9-.35*event.progress},{id:to,intensity:arrival}];
}
