import { readRepository, sourceFiles, encodeBytes } from './repository.js';
import { classifyInput, droppedLink, droppedFiles, fetchMagic, INPUT_HELP } from './input.js';
import { languageColors } from './languages.js';
import { LivingInk } from './effects.js';
import { Quill, quillCadence, followingView, glide } from './quill.js';
import { validateSvg } from './svg.js';
const $ = id => document.getElementById(id);
let livingInk=new LivingInk();
const reducedMotion=matchMedia('(prefers-reduced-motion: reduce)');
// The quill writes each new diagram onto the page; it waits until the page is in view.
let quill=null,writingDoneAt=-Infinity;
// The view eases toward the seal being written ('following'), then back to the fitted page ('homing'), until the reader takes the view in hand (null).
let penFollowing=null,penVelocity={zoom:0,panX:0,panY:0},lastPenGlide=0;
const penGlideSeconds=.8;
function takeHand(){penFollowing=null;}
let glowing=true,moving=!reducedMotion.matches,animationFrame=null,animationEpoch=performance.now(),lastAnimationFrame=0,previewVisible=true;
const canvas = $('canvas'), ctx = canvas.getContext('2d'), viewport = $('viewport');
// The circle chamber is a scroll: across on wide screens, unrolling downward on phones.
const scrollCase = document.querySelector('.grimoire-scroll'), narrowScroll = matchMedia('(max-width: 760px)');
let worker, requestId = 0, pending = new Map(), image, diagram, files = [], title = 'Code is Magic';
let snapshot = null, inputSelection = null;
let diagramKey = '';
const resultKeys = new WeakMap();
const resultPngs = new WeakMap();
const cachedPngs = new WeakMap();
let zoom = 1, panX = 0, panY = 0, selected = null, controller = null, busy = false, imported = false, reframe = true;
function status(message, type = '') { $('status').textContent = message; $('status').className = `status ${type}`; }
function setBusy(value) { if(value)clearTimeout(debounce);busy = value;for(const id of ['download-magic','download-png','download-svg','download-svgz','download-json','decode-names'])$(id).disabled=value||!diagram; for(const id of ['magic-key','generate-key','save-key','clear-key','palette','min-lines','wingdings','language-colors','theme-toggle'])$(id).disabled=value; $('generate').disabled = value; $('choose-image').disabled = value; $('repo').readOnly = value; $('demo').disabled = value; $('cancel').hidden = !value; $('progress').hidden = !value; }
function getWorker() {
  if (!worker) {
    worker = new Worker(new URL('./worker.js', import.meta.url), { type: 'module' });
    worker.onmessage = ({data}) => {
      const p = pending.get(data.id); if (!p) return;
      pending.delete(data.id); data.error ? p.reject(new Error(data.error)) : p.resolve(data.result);
    };
    worker.onerror = () => { for (const p of pending.values()) p.reject(new Error('The Rust engine could not load. Build web/pkg with scripts/build-web.sh and serve the site over HTTP.')); pending.clear(); worker.terminate(); worker = null; };
  }
  return worker;
}
async function engine(data) {
  const w=getWorker(),id=++requestId;
  return new Promise((resolve,reject)=>{pending.set(id,{resolve,reject});w.postMessage({id,...data});});
}
async function transmute(inputFiles,inputTitle,inputSnapshot=snapshot) {
  const key=$('magic-key').value.trim();
  const result=await engine({key,files:inputFiles,title:inputTitle,minLines:Number($('min-lines').value),palette:$('palette').value,wingdings:$('wingdings').checked,languageColors:$('language-colors').checked});
  resultKeys.set(result,key);result.snapshot=inputSnapshot;return result;
}
function capsule() {
  if(!snapshot)throw new Error('This old diagram has no full repository snapshot. Load a repository or source folder first.');
  return {version:1,snapshot,svg:diagram.svg};
}
async function install(next,signal,source) {
  // Generated SVG has no external images or executable source text.
  const blob = new Blob([next.svg], {type:'image/svg+xml'}), url = URL.createObjectURL(blob);
  const nextImage = new Image();
  try { await new Promise((resolve,reject) => { nextImage.onload = resolve; nextImage.onerror = () => reject(new Error('The inscription image could not render.')); nextImage.src = url; }); }
  finally { URL.revokeObjectURL(url); }
  const nextInk=new LivingInk();await nextInk.prepare(next.svg,next);
  signal?.throwIfAborted();
  if(source){snapshot=source.snapshot;files=source.files;title=source.title;imported=false;reframe=true;}
  quill=reducedMotion.matches?null:new Quill(next,nextInk,{fast:!reframe,now:Infinity,axis:narrowScroll.matches?'down':'across'});writingDoneAt=quill?-Infinity:performance.now()-1e4;writingState();
  livingInk=nextInk;animationEpoch=performance.now();diagram = next; diagramKey=resultKeys.get(next)||''; image = nextImage; selected = null;
  viewport.dataset.ink=livingInk.background==='#f0e8d7'?'light':'dark';
  scheduleAnimation();
  if (reframe) { shapeScroll(); fit(); } else draw();
  penFollowing=quill&&!quill.fast?'following':null;penVelocity={zoom:0,panX:0,panY:0};
  $('diagram-title').textContent = title;
  const visible = next.analysis.entities.filter(e=>e.kind!=='merged_impl');
  const calls = next.analysis.edges.filter(e=>e.kind==='calls').length;
  $('canvas-stats').textContent = `${visible.length} SEALS · ${calls} CALL CHANNELS`;
  $('coverage').textContent = `${next.analysis.files} visualized source files · ${snapshot?.entries.length||0} entries preserved`;
  canvas.setAttribute('aria-label', `${visible.length} magic seals with ${calls} call channels for ${title}`);
  for (const id of ['download-magic','download-png','download-svg','download-svgz','download-json']) $(id).disabled = false;
  listEntities(); inspect(null);
}
function draw(time=performance.now()-animationEpoch) {
  const bounds = viewport.getBoundingClientRect(), dpr = Math.min(devicePixelRatio || 1, 2);
  const w = Math.round(bounds.width*dpr), h = Math.round(bounds.height*dpr);
  if(w<1||h<1)return;
  if (canvas.width!==w || canvas.height!==h) { canvas.width=w;canvas.height=h; }
  ctx.setTransform(dpr,0,0,dpr,0,0);ctx.clearRect(0,0,bounds.width,bounds.height);
  if (!image) return;
  const now=performance.now();
  if(quill&&quill.done(now))finishWriting(now);
  followPen(now,bounds);
  unrollScroll(quill?quill.unrolled(now,{panX,panY,zoom,width:bounds.width,height:bounds.height}):1);
  const awaken=Math.max(0,Math.min(1,(now-writingDoneAt)/900));
  livingInk.paint(ctx,image,diagram,{width:bounds.width,height:bounds.height,dpr,panX,panY,zoom},typeof time==='number'?time:now-animationEpoch,{glow:glowing,motion:moving&&!quill,writing:quill,now,awaken});
  if (selected!==null) { const p=diagram.placements.find(p=>p.id===selected);if(p){ctx.beginPath();ctx.arc(panX+p.x*zoom,panY+p.y*zoom,(p.radius+9)*zoom,0,Math.PI*2);ctx.strokeStyle=livingInk.background==='#f0e8d7'?'#73502d':'#f5dfaa';ctx.lineWidth=1.5;ctx.stroke();} }
  const flowText=quill?(Number.isFinite(quill.begin)?`The quill inscribes ${quill.written(now)} of ${quill.plan.seals.size} seals…`:'The page awaits the quill'):moving?livingInk.describe(typeof time==='number'?time:performance.now()-animationEpoch):'Call flow paused';
  if($('flow-label').textContent!==flowText)$('flow-label').textContent=flowText;
  const zoomText=`${Math.round(zoom*100)}%`;if($('zoom-label').textContent!==zoomText)$('zoom-label').textContent=zoomText;
}
// Ease the view toward the seal under the quill, or home to the fitted page once writing is over.
function followPen(now,view){
  if(!penFollowing||!image)return;
  const hourglassSeconds=Math.min(.1,Math.max(0,(now-lastPenGlide)/1000));lastPenGlide=now;
  const fitted=fittedView(view),seal=penFollowing==='following'&&quill&&Number.isFinite(quill.begin)&&quill.current(now);
  const aim=penFollowing==='homing'?fitted:seal&&followingView(seal,fitted,view);
  if(!aim)return;
  const glidedZoom=glide(Math.log(zoom),penVelocity.zoom,Math.log(aim.zoom),hourglassSeconds,penGlideSeconds),glidedX=glide(panX,penVelocity.panX,aim.panX,hourglassSeconds,penGlideSeconds),glidedY=glide(panY,penVelocity.panY,aim.panY,hourglassSeconds,penGlideSeconds);
  zoom=Math.exp(glidedZoom.value);panX=glidedX.value;panY=glidedY.value;penVelocity={zoom:glidedZoom.velocity,panX:glidedX.velocity,panY:glidedY.velocity};
  if(penFollowing==='homing'&&Math.abs(Math.log(zoom/aim.zoom))<.003&&Math.hypot(panX-aim.panX,panY-aim.panY)<.5){zoom=aim.zoom;panX=aim.panX;panY=aim.panY;penFollowing=null;}
}
// Size the parchment to the diagram's own shape, so the drawing reaches close to its edges.
function shapeScroll() {
  if(!diagram)return;
  const bounds=livingInk.bounds,bearing=Math.max(.45,Math.min(1.6,(bounds.bottom-bounds.top)/Math.max(1,bounds.right-bounds.left)));
  const rodBreadth=parseFloat(getComputedStyle(scrollCase).getPropertyValue('--rod-breadth'))||30;
  const caseWidth=scrollCase.parentElement.clientWidth;
  let parchmentWidth,parchmentHeight;
  if(narrowScroll.matches) {
    parchmentWidth=caseWidth-44;
    parchmentHeight=Math.max(360,Math.min(parchmentWidth*bearing,innerHeight-150));
  } else {
    const openWidth=caseWidth-rodBreadth,tallest=Math.max(440,innerHeight-80);
    parchmentHeight=Math.min(openWidth*bearing,tallest);
    parchmentWidth=Math.max(Math.min(openWidth,parchmentHeight/bearing),Math.min(openWidth,520));
  }
  viewport.style.width=`${Math.round(parchmentWidth)}px`;viewport.style.height=`${Math.round(parchmentHeight)}px`;
}
// Rods start together at the heart of the scroll and part as the quill writes.
let lastUnrolling=-1;
function unrollScroll(unrolling) {
  const rodBreadth=parseFloat(getComputedStyle(scrollCase).getPropertyValue('--rod-breadth'))||30;
  const parchmentSpan=narrowScroll.matches?viewport.clientHeight:viewport.clientWidth;
  const shut=Math.min(1,rodBreadth/Math.max(1,parchmentSpan));
  const opening=Math.max(shut,unrolling);
  if(Math.abs(opening-lastUnrolling)<.0005)return;
  lastUnrolling=opening;scrollCase.style.setProperty('--unroll',opening.toFixed(4));
}
let scrollResize;
addEventListener('resize',()=>{clearTimeout(scrollResize);scrollResize=setTimeout(shapeScroll,120);},{passive:true});
narrowScroll.addEventListener('change',shapeScroll);
function fit() {
  if(!diagram)return;
  const fitted=fittedView();zoom=fitted.zoom;panX=fitted.panX;panY=fitted.panY;draw();
}
function fittedView(view=viewport.getBoundingClientRect()) {
  const bounds=livingInk.bounds;
  const width=Math.max(1,bounds.right-bounds.left),height=Math.max(1,bounds.bottom-bounds.top);
  // A modest margin: the worn edge, plus the rolls that rest over the parchment's ends.
  const margin=Math.max(10,Math.min(26,Math.min(view.width,view.height)*.03))+12;
  const fittedZoom=Math.min((view.width-margin*2)/width,(view.height-margin*2)/height,8);
  return {zoom:fittedZoom,panX:view.width/2-(bounds.left+width/2)*fittedZoom,panY:view.height/2-(bounds.top+height/2)*fittedZoom};
}
function zoomAt(factor,x,y) {takeHand();const z=Math.min(8,Math.max(.025,zoom*factor));panX=x-(x-panX)*z/zoom;panY=y-(y-panY)*z/zoom;zoom=z;draw();}
viewport.addEventListener('wheel',e=>{e.preventDefault();const b=viewport.getBoundingClientRect();zoomAt(Math.exp(-e.deltaY*.001),e.clientX-b.left,e.clientY-b.top);},{passive:false});
let drag;
viewport.addEventListener('pointerdown',touch=>{takeHand();if(quill)finishWriting();drag={x:touch.clientX,y:touch.clientY,px:panX,py:panY,moved:false};viewport.setPointerCapture(touch.pointerId);});
viewport.addEventListener('pointermove',e=>{if(!drag)return;const dx=e.clientX-drag.x,dy=e.clientY-drag.y;drag.moved ||= Math.hypot(dx,dy)>4;panX=drag.px+dx;panY=drag.py+dy;draw();});
viewport.addEventListener('pointerup',e=>{if(!drag)return;if(!drag.moved&&diagram){const b=viewport.getBoundingClientRect(),x=(e.clientX-b.left-panX)/zoom,y=(e.clientY-b.top-panY)/zoom;const p=diagram.placements.find(p=>Math.hypot(p.x-x,p.y-y)<p.radius);if(p)inspect(p.id);}drag=null;});
viewport.addEventListener('pointercancel',()=>{drag=null;});
viewport.addEventListener('keydown',keystroke=>{if(quill&&!['Shift','Control','Alt','Meta','Tab'].includes(keystroke.key)){finishWriting();if(keystroke.key===' '||keystroke.key==='Enter'||keystroke.key==='Escape'){keystroke.preventDefault();return;}}if(['ArrowLeft','ArrowRight','ArrowUp','ArrowDown'].includes(keystroke.key)){takeHand();keystroke.preventDefault();panX+=keystroke.key==='ArrowLeft'?30:keystroke.key==='ArrowRight'?-30:0;panY+=keystroke.key==='ArrowUp'?30:keystroke.key==='ArrowDown'?-30:0;draw();}if(keystroke.key==='+'||keystroke.key==='=')$('zoom-in').click();if(keystroke.key==='-')$('zoom-out').click();});
let lastViewportWidth = 0, lastViewportHeight = 0;
new ResizeObserver(() => { const bounds = viewport.getBoundingClientRect(); if (bounds.width !== lastViewportWidth || bounds.height !== lastViewportHeight) { lastViewportWidth = bounds.width; lastViewportHeight = bounds.height; fit(); } else { draw(); } }).observe(viewport);
$('fit').onclick=()=>{takeHand();fit();};
for(const [id,factor] of [['zoom-in',1.25],['zoom-out',.8]])$(id).onclick=()=>{const b=viewport.getBoundingClientRect();zoomAt(factor,b.width/2,b.height/2);};
function listEntities() {
  const root=$('entities');root.replaceChildren();if(!diagram)return;
  const query=$('search').value.toLowerCase();
  for(const e of diagram.analysis.entities.filter(e=>e.kind!=='merged_impl'&&`${e.qualified_name} ${e.path} ${e.language}`.toLowerCase().includes(query))) {
    const button=document.createElement('button');button.type='button';button.dataset.id=e.id;button.classList.toggle('selected',selected===e.id);
    button.append(document.createTextNode(`${e.kind==='function'?'◎':e.kind==='struct'?'⬡':'◇'} ${e.qualified_name}`));
    const detail=document.createElement('small');detail.textContent=`${e.language} · ${e.path}:${e.line} · ${e.lines} lines`;button.append(detail);button.onclick=()=>{takeHand();inspect(e.id);const p=diagram.placements.find(p=>p.id===e.id);if(p){const b=viewport.getBoundingClientRect();zoom=Math.max(zoom,1);panX=b.width/2-p.x*zoom;panY=b.height/2-p.y*zoom;draw();}};root.append(button);
  }
}
$('search').oninput=listEntities;
function inspect(id) {
  selected=id;draw();const root=$('inspector');root.replaceChildren();
  const heading=document.createElement('p');heading.className='eyebrow';heading.textContent='READ THE GRIMOIRE';root.append(heading);
  const h=document.createElement('h3');root.append(h);
  if(id===null){h.textContent='Select a seal';const p=document.createElement('p');p.textContent='Inspect its language, source location, syntax, and outgoing calls.';root.append(p);return;}
  const e=diagram.analysis.entities.find(e=>e.id===id);h.textContent=e.qualified_name;
  const dl=document.createElement('dl');for(const [key,value] of [['Language',e.language],['Structure',e.kind],['Source',`${e.path}:${e.line}–${e.end_line}`],['Size',`${e.lines} non-comment source lines`],['Calls',e.calls.length?e.calls.join(', '):'No calls found'],['Fields',e.fields.length?e.fields.join(', '):'—']]){const dt=document.createElement('dt'),dd=document.createElement('dd');dt.textContent=key;dd.textContent=value;dl.append(dt,dd);}root.append(dl);
  const caption=document.createElement('p');caption.textContent='Syntax sampled around the outer ring:';const tokens=document.createElement('p');tokens.className='token-sample';tokens.textContent=e.tokens.slice(0,90).join(' ');root.append(caption,tokens);
  for(const button of $('entities').children)button.classList.toggle('selected',Number(button.dataset.id)===id);
}
function download(blob,extension) {const url=URL.createObjectURL(blob),a=document.createElement('a');a.href=url;a.download=`${title.split(' @ ')[0].replace(/[^a-zA-Z0-9_-]/g,'-') || 'code-is-magic'}.${extension}`;a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);}
$('download-svg').onclick=async()=>{try{setBusy(true);download(new Blob([await engine({op:'magic-svg',capsule:capsule(),key:diagramKey})],{type:'image/svg+xml'}),'svg');status('Magic circles downloaded as plain SVG. It opens from disk in any browser; the original source is sealed inside.');}catch(e){status(e.message,'error');}finally{setBusy(false);}};
const downloadMagic=async()=>{try{setBusy(true);download(new Blob([await engine({op:'svgz',capsule:capsule(),key:diagramKey})],{type:'application/gzip'}),'svgz');status('Magic circles downloaded as SVGZ: a compact scripted drawing that unfolds in the browser, with the original source sealed inside.');}catch(e){status(e.message,'error');}finally{setBusy(false);}};
$('download-magic').onclick=downloadMagic;
$('download-svgz').onclick=downloadMagic;
$('download-json').onclick=async()=>{
  try{setBusy(true);const data=diagramKey?{version:2,svg:await engine({op:'svg',capsule:capsule(),key:diagramKey})}:{...diagram,snapshot};download(new Blob([JSON.stringify(data)],{type:'application/json'}),'json');status('Offline diagram downloaded. Keyed diagrams require the same key on import.');}
  catch(e){status(e.message,'error');}finally{setBusy(false);}
};
$('download-png').onclick=async()=>{
  try {setBusy(true);status('Compressing recoverable PNG without reducing fidelity…','busy');const original=resultPngs.get(diagram);const bytes=cachedPngs.get(diagram)||(original?await engine({op:'optimize-png',bytes:original}):await engine({op:'png',capsule:capsule(),key:diagramKey}));cachedPngs.set(diagram,bytes);download(new Blob([bytes],{type:'image/png'}),'png');status(original?'Compressed PNG downloaded. Every original pixel and source bit preserved.':'Recoverable PNG downloaded. Keep the original PNG without resizing or JPEG conversion.');}
  catch(e){status(e.message,'error');}finally{setBusy(false);}
};
function restoreArtwork(next,svg) {
    const original=new DOMParser().parseFromString(svg,'image/svg+xml').documentElement;
    const width=Number(original.getAttribute('width')),height=Number(original.getAttribute('height'));
    if(!Number.isFinite(width)||!Number.isFinite(height)||width<=0||height<=0)throw new Error('Invalid original artwork dimensions.');
    next.svg=svg;next.width=width;next.height=height;
    next.placements=[...original.querySelectorAll('polygon[data-name-sigil="1"]')].map(node=>{
      const match=node.getAttribute('transform')?.match(/^translate\(([-\d.]+) ([-\d.]+)\) scale\(([-\d.]+)\)$/);
      if(!match)throw new Error('Invalid original name contour transform.');
      const radius=Number(node.getAttribute('data-seal-radius')??match[3]);
      if(!Number.isFinite(radius)||radius<=0)throw new Error('Invalid original seal radius.');
      return {id:Number(node.getAttribute('data-entity')),x:Number(match[1]),y:Number(match[2]),radius};
    });
    const background=original.querySelector('rect')?.getAttribute('fill');
    $('palette').value=background==='#f0e8d7'?'parchment':background==='#090d20'?'astral':'gold';
    $('language-colors').checked=original.getAttribute('data-language-colors')==='true';colorLegend();
    resultKeys.set(next,original.getAttribute('data-magic-key')==='1'?$('magic-key').value.trim():'');
}
async function recoverSource(file,signal) {
  status('Recovering exact repository bytes from the image…','busy');
  if(file.size>150_000_000)throw new Error('Image exceeds the upload budget.');
  const originalBytes=new Uint8Array(await file.arrayBuffer());signal.throwIfAborted();
  const recovered=await engine({op:'recover',key:$('magic-key').value.trim(),bytes:originalBytes});signal.throwIfAborted();
  const zip=await engine({op:'zip',snapshot:recovered.snapshot});signal.throwIfAborted();
  const nextFiles=sourceFiles(recovered.snapshot),next=await transmute(nextFiles,recovered.snapshot.title,recovered.snapshot);signal.throwIfAborted();
  restoreArtwork(next,recovered.svg);
  if(originalBytes[0]===137&&originalBytes[1]===80)resultPngs.set(next,originalBytes);
  await install(next,signal,{snapshot:recovered.snapshot,files:nextFiles,title:recovered.snapshot.title});signal.throwIfAborted();
  download(new Blob([zip],{type:'application/zip'}),'zip');
  status(`Recovered ${snapshot.entries.length} tracked entries with exact bytes. Source ZIP downloaded.`);
}
function inputMode() {
  const recovery=inputSelection?.kind==='image';
  $('input-mark').textContent=recovery?'◇':'⌘';
  $('input-help').textContent=inputSelection?`${inputSelection.label} ready. Open grimoire to ${recovery?'recover its exact source ZIP':'inscribe its source'}. Edit this field to replace it with a link.`:INPUT_HELP;
}
function chooseInput(value) {
  if(busy)return;
  try {
    const next=classifyInput(value);inputSelection=typeof value==='string'?null:next;
    $('repo').value=next.label;inputMode();status('Grimoire ready. Select Open grimoire to begin.');$('generate').focus();return true;
  }catch(error){status(error.message,'error');return false;}
}
$('choose-image').onclick=()=>$('restore').click();
$('restore').onchange=event=>{if(event.target.files.length)chooseInput(event.target.files);event.target.value='';};
$('repo').oninput=()=>{inputSelection=null;inputMode();};
$('repo-form').addEventListener('paste',event=>{
  const files=[...(event.clipboardData?.files||[])];if(files.length){event.preventDefault();chooseInput(files);}
});
$('repo-form').addEventListener('dragover',event=>{
  if([...event.dataTransfer?.types||[]].some(type=>['Files','text/uri-list','text/plain'].includes(type))){event.preventDefault();if(!busy)$('repo-form').classList.add('drop-ready');}
});
$('repo-form').addEventListener('dragleave',event=>{if(!$('repo-form').contains(event.relatedTarget))$('repo-form').classList.remove('drop-ready');});
$('repo-form').addEventListener('drop',async event=>{
  event.preventDefault();$('repo-form').classList.remove('drop-ready');if(busy)return;
  // Capture browser file-entry handles synchronously, before the drag store closes.
  const transfer=event.dataTransfer,link=droppedLink(transfer);
  try {
    if(transfer.files.length) {const reading=droppedFiles(transfer);setBusy(true);try{const inputs=await reading;setBusy(false);chooseInput(inputs);}finally{setBusy(false);}}
    else if(link)chooseInput(link);
  }catch(error){status(error.message,'error');}
});
$('decode-names').onclick=async()=>{
  try{const recovered=await engine({op:'names',svg:diagram.svg,key:diagramKey});status(`Decoded ${recovered.length} exact names from contour geometry.`);if(selected!==null){const value=recovered.find(e=>e.entity_id===selected);if(value)$('inspector').querySelector('h3').textContent=value.name;}}
  catch(e){status(e.message,'error');}
};
$('access-button').onclick=()=>{$('access').open=!$('access').open;};
async function openInput(selection) {
  if(busy)return;
  controller=new AbortController();const activeController=controller,{signal}=activeController;
  setBusy(true);status('Opening the source grimoire…','busy');
  const token=$('token').value;$('token').value='';
  try {
    if(selection.kind==='image'||selection.kind==='image-link') {
      const file=selection.file||await fetchMagic(selection.url,{signal});signal.throwIfAborted();
      await recoverSource(file,signal);return;
    }
    let result;
    if(selection.kind==='default') {
      const response=await fetch(new URL('./default-project.json',import.meta.url),{signal});
      if(!response.ok)throw new Error('The bundled Code is Magic grimoire is missing. Build the website with scripts/build-web.sh.');
      const nextSnapshot=await response.json();
      result={snapshot:nextSnapshot,files:sourceFiles(nextSnapshot),title:nextSnapshot.title,commit:nextSnapshot.commit};
    } else if(selection.kind==='source') {
      const entries=[];
      for(const file of selection.files) {
        signal.throwIfAborted();
        const path=file.webkitRelativePath?.split('/').slice(1).join('/')||file.sourcePath||file.name;
        entries.push({path,mode:'100644',data:encodeBytes(new Uint8Array(await file.arrayBuffer())),sha:null});
      }
      entries.sort((a,b)=>a.path.localeCompare(b.path));
      if(new Set(entries.map(e=>e.path)).size!==entries.length)throw new Error('Source files have duplicate paths. Choose their containing folder.');
      const nextTitle=`Local grimoire · ${selection.label}`,nextSnapshot={version:1,title:nextTitle,commit:null,entries};
      result={snapshot:nextSnapshot,files:sourceFiles(nextSnapshot),title:nextTitle,commit:null};
    } else {
      result=await readRepository(selection.url,{reference:$('reference').value,token,signal,progress:(message,current,total)=>{if(activeController!==controller)return;status(message,'busy');$('progress').max=total||1;$('progress').value=current;}});
    }
    signal.throwIfAborted();status('Inking syntax and connecting seals…','busy');
    const next=await transmute(result.files,result.title,result.snapshot);signal.throwIfAborted();
    await install(next,signal,result);signal.throwIfAborted();
    status(`Inscribed ${result.files.length} source files; preserved ${result.snapshot.entries.length} entries${result.commit?` at ${result.commit.slice(0,12)}`:''}. Light rehearses resolved calls and returns.`);
  } catch(error) {if(!signal.aborted){activeController.abort();status(error.message,'error');}}
  finally {if(activeController===controller){setBusy(false);controller=null;}}
}
$('repo-form').onsubmit=async event=>{
  event.preventDefault();if(busy)return;
  try{await openInput(inputSelection||classifyInput($('repo').value));}catch(error){status(error.message,'error');}
};
$('cancel').onclick=()=>{controller?.abort();if(worker){worker.terminate();worker=null;for(const p of pending.values())p.reject(new Error('Cancelled'));pending.clear();}status('Transmutation cancelled. The previous inscription remains.');};
function loadDefault(){if(busy)return;inputSelection=null;$('repo').value='https://github.com/wunderay/Code-Is-Magic';inputMode();return openInput({kind:'default'});}
$('demo').onclick=loadDefault;
let debounce;
function rerender(){if(busy)return;if(imported){status('Imported diagrams preserve their original styling. Open source files to change the inscription settings.');return;}clearTimeout(debounce);status('Updating inscription…','busy');debounce=setTimeout(async()=>{if(busy)return;setBusy(true);try{reframe=false;await install(await transmute(files,title));status('Inscription updated.');}catch(e){status(e.message,'error');}finally{setBusy(false);}},150);}
$('min-lines').oninput=()=>{$('threshold-value').textContent=`${$('min-lines').value} lines`;rerender();};$('palette').onchange=rerender;$('wingdings').onchange=rerender;
function colorLegend() {$('language-colors').title='Language hues adapt to the selected background';document.querySelector('.legend').classList.toggle('language-colors',$('language-colors').checked);}
for(const label of document.querySelectorAll('.legend dt[data-language]')){const dot=document.createElement('span');dot.className='language-dot';dot.setAttribute('aria-hidden','true');dot.style.backgroundColor=languageColors[label.dataset.language];label.prepend(dot);}
$('language-colors').onchange=()=>{colorLegend();rerender();};
$('folder').onchange=event=>{
  if(busy)return;
  const inputs=[...event.target.files].filter(f=>!/(^|\/)(\.git|target|node_modules|pkg)\//.test(f.webkitRelativePath));
  if(inputs.length&&chooseInput(inputs))$('repo-form').requestSubmit();event.target.value='';
};
$('json').onchange=async event=>{if(busy)return;const file=event.target.files[0];if(!file)return;setBusy(true);try{if(file.size>50_000_000)throw new Error('Diagram is too large.');let next=JSON.parse(await file.text());if(next.version===2&&typeof next.svg==='string'){
      const recovered=await engine({op:'recover',key:$('magic-key').value.trim(),bytes:new TextEncoder().encode(next.svg)});
      // Work on recovered bytes only after successful key authentication.
      next=await transmute(sourceFiles(recovered.snapshot),recovered.snapshot.title);
      next.snapshot=recovered.snapshot;restoreArtwork(next,recovered.svg);
    }if(!next.analysis?.entities||!Array.isArray(next.placements)||typeof next.svg!=='string'||!Number.isFinite(next.width)||!Number.isFinite(next.height)||next.width<1||next.height<1||next.width>100000||next.height>100000)throw new Error('Choose a JSON diagram exported by Code Is Magic.');const document=new DOMParser().parseFromString(next.svg,'image/svg+xml');validateSvg(document);$('language-colors').checked=document.documentElement.getAttribute('data-language-colors')==='true';colorLegend();title=`Imported grimoire · ${file.name}`;snapshot=next.snapshot||null;reframe=true;await install(next);imported=true;status('Offline diagram opened. Original styling preserved.');}catch(e){status(e.message,'error');}finally{setBusy(false);event.target.value='';}};
function updateKey(){if(busy)return;if($('magic-key').value.trim()===diagramKey)return;if(imported&&snapshot){files=sourceFiles(snapshot);imported=false;}rerender();}
$('magic-key').onchange=updateKey;
$('generate-key').onclick=()=>{
  const bytes=crypto.getRandomValues(new Uint8Array(32));
  $('magic-key').value=Array.from(bytes,b=>b.toString(16).padStart(2,'0')).join('');
  updateKey();
};
$('save-key').onclick=()=>{
  const key=$('magic-key').value.trim();
  if(!/^[0-9a-fA-F]{64}$/.test(key)){status('Enter or generate a 64-digit hexadecimal key first.','error');return;}
  const url=URL.createObjectURL(new Blob([key+'\n'],{type:'text/plain'}));
  const a=document.createElement('a');a.href=url;a.download='code-is-magic.key';a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);
  status('Key saved. Keep it separate from the source folder and image.');
};
$('clear-key').onclick=()=>{$('magic-key').value='';updateKey();};
function finishWriting(now=performance.now()){
  if(!quill)return;
  penFollowing=penFollowing==='following'?'homing':null;
  quill.finish();quill=null;writingDoneAt=now;animationEpoch=now;writingState();draw();scheduleAnimation();
}
function writingState(){
  $('quill-skip').hidden=!quill;
  viewport.dataset.writing=quill?'true':'false';
}
function animating(){return moving||!!quill||penFollowing==='homing'||performance.now()-writingDoneAt<900;}
function scheduleAnimation(){
  if(animationFrame!==null||!animating()||!image||document.hidden)return;
  const bounds=viewport.getBoundingClientRect();
  previewVisible=bounds.width>0&&bounds.height>0&&bounds.bottom>0&&bounds.top<innerHeight&&bounds.right>0&&bounds.left<innerWidth;
  if(!previewVisible){canvas.dataset.motion='paused';return;}
  // Begin writing once a fair part of the scroll can be seen. Until then the waiting page never
  // changes, so no frame is queued; scrolling, resizing or the sight watcher below wakes the loop.
  const cadence=quillCadence(quill,Math.min(bounds.bottom,innerHeight)-Math.max(bounds.top,0),{fullView:Math.min(bounds.height,innerHeight),fast:quill?.fast});
  if(cadence==='rest'){canvas.dataset.motion='paused';return;}
  if(cadence==='begin')quill.begin=performance.now();
  canvas.dataset.motion='running';
  animationFrame=requestAnimationFrame(timestamp=>{
    animationFrame=null;
    if(timestamp-lastAnimationFrame>=(quill||penFollowing?15:33)){lastAnimationFrame=timestamp;draw(timestamp-animationEpoch);}
    scheduleAnimation();
  });
}
function displayOptions(){
  $('glow-toggle').setAttribute('aria-pressed',String(glowing));
  $('motion-toggle').setAttribute('aria-pressed',String(moving));
  if(!animating()&&animationFrame!==null){cancelAnimationFrame(animationFrame);animationFrame=null;}
  canvas.dataset.motion=moving?'running':'paused';draw();scheduleAnimation();
}
$('glow-toggle').onclick=()=>{glowing=!glowing;displayOptions();};
$('motion-toggle').onclick=()=>{moving=!moving;displayOptions();};
document.addEventListener('visibilitychange',()=>{if(document.hidden&&animationFrame!==null){cancelAnimationFrame(animationFrame);animationFrame=null;}else scheduleAnimation();});
window.addEventListener('scroll',scheduleAnimation,{passive:true});
// Wakes the loop when the scroll comes into view without a scroll event, e.g. after the layout shifts.
if('IntersectionObserver' in window)new IntersectionObserver(scheduleAnimation,{threshold:[0,.05,.1,.2,.3,.4,.5,.6,.8,1]}).observe(viewport);
window.addEventListener('resize',scheduleAnimation,{passive:true});
reducedMotion.addEventListener('change',event=>{if(event.matches){finishWriting();moving=false;displayOptions();}});
$('quill-skip').onclick=()=>{finishWriting();viewport.focus();};
document.addEventListener('keydown',keystroke=>{if(keystroke.key==='Escape'&&quill)finishWriting();});
$('settings-toggle').onclick=()=>{const open=!document.querySelector('.workbench').hasAttribute('data-settings-open');document.querySelector('.workbench').toggleAttribute('data-settings-open',open);$('settings-toggle').setAttribute('aria-expanded',String(open));};
function theme(mode,changeInk=false){
  document.documentElement.dataset.theme=mode;
  $('theme-label').textContent=mode==='light'?'Dark mode':'Light mode';
  $('theme-toggle').setAttribute('aria-pressed',String(mode==='light'));
  document.querySelector('meta[name="theme-color"]').content=mode==='light'?'#efe9db':'#121317';
  try{localStorage.setItem('code-is-magic-theme',mode);}catch{}
  if(changeInk&&!imported){$('palette').value=mode==='light'?'parchment':'gold';rerender();}
}
$('theme-toggle').onclick=()=>theme(document.documentElement.dataset.theme==='light'?'dark':'light',true);
let savedTheme='dark';try{if(localStorage.getItem('code-is-magic-theme')==='light')savedTheme='light';}catch{}
theme(savedTheme);if(savedTheme==='light')$('palette').value='parchment';displayOptions();
colorLegend();inputMode();loadDefault();
