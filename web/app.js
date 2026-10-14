import { readRepository, SOURCE_PATTERN, sourceFiles, encodeBytes, snapshotFromSources } from './repository.js';
import { demoFiles } from './demo.js';
import { languageColors } from './languages.js';
import { LivingInk } from './effects.js';
const $ = id => document.getElementById(id);
let livingInk=new LivingInk();
let glowing=true,moving=!matchMedia('(prefers-reduced-motion: reduce)').matches,animationFrame=null,animationEpoch=performance.now(),lastAnimationFrame=0,previewVisible=true;
const canvas = $('canvas'), ctx = canvas.getContext('2d'), viewport = $('viewport');
let worker, requestId = 0, pending = new Map(), image, diagram, files = demoFiles, title = 'Example grimoire · nine language signatures';
let snapshot = snapshotFromSources(demoFiles,title);
let inputImage = null;
let diagramKey = '';
const resultKeys = new WeakMap();
let zoom = 1, panX = 0, panY = 0, selected = null, controller = null, busy = false, imported = false, reframe = true;
function status(message, type = '') { $('status').textContent = message; $('status').className = `status ${type}`; }
function setBusy(value) { if(value)clearTimeout(debounce);busy = value;for(const id of ['download-png','download-svg','download-json','decode-names'])$(id).disabled=value||!diagram; for(const id of ['magic-key','generate-key','save-key','clear-key','palette','min-lines','wingdings','language-colors','theme-toggle'])$(id).disabled=value; $('generate').disabled = value; $('choose-image').disabled = value; $('repo').readOnly = value; $('demo').disabled = value; $('cancel').hidden = !value; $('progress').hidden = !value; }
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
async function transmute(inputFiles,inputTitle) {
  const key=$('magic-key').value.trim();
  const result=await engine({key,files:inputFiles,title:inputTitle,minLines:Number($('min-lines').value),palette:$('palette').value,wingdings:$('wingdings').checked,languageColors:$('language-colors').checked});
  resultKeys.set(result,key);result.snapshot=snapshot;return result;
}
function capsule() {
  if(!snapshot)throw new Error('This old diagram has no full repository snapshot. Load a repository or source folder first.');
  return {version:1,snapshot,svg:diagram.svg};
}
async function install(next) {
  // Generated SVG has no external images or executable source text.
  const blob = new Blob([next.svg], {type:'image/svg+xml'}), url = URL.createObjectURL(blob);
  const nextImage = new Image();
  try { await new Promise((resolve,reject) => { nextImage.onload = resolve; nextImage.onerror = () => reject(new Error('The inscription image could not render.')); nextImage.src = url; }); }
  finally { URL.revokeObjectURL(url); }
  const nextInk=new LivingInk();await nextInk.prepare(next.svg);
  livingInk=nextInk;animationEpoch=performance.now();diagram = next; diagramKey=resultKeys.get(next)||''; image = nextImage; selected = null;
  scheduleAnimation();
  if (reframe) fit(); else draw();
  $('diagram-title').textContent = title;
  const visible = next.analysis.entities.filter(e=>e.kind!=='merged_impl');
  const calls = next.analysis.edges.filter(e=>e.kind==='calls').length;
  $('canvas-stats').textContent = `${visible.length} SEALS · ${calls} CALL CHANNELS`;
  $('coverage').textContent = `${next.analysis.files} visualized source files · ${snapshot?.entries.length||0} entries preserved`;
  canvas.setAttribute('aria-label', `${visible.length} magic seals with ${calls} call channels for ${title}`);
  for (const id of ['download-png','download-svg','download-json']) $(id).disabled = false;
  listEntities(); inspect(null);
}
function draw(time=performance.now()-animationEpoch) {
  const bounds = viewport.getBoundingClientRect(), dpr = Math.min(devicePixelRatio || 1, 2);
  const w = Math.round(bounds.width*dpr), h = Math.round(bounds.height*dpr);
  if(w<1||h<1)return;
  if (canvas.width!==w || canvas.height!==h) { canvas.width=w;canvas.height=h; }
  ctx.setTransform(dpr,0,0,dpr,0,0);ctx.clearRect(0,0,bounds.width,bounds.height);
  if (!image) return;
  livingInk.paint(ctx,image,diagram,{width:bounds.width,height:bounds.height,dpr,panX,panY,zoom},typeof time==='number'?time:performance.now()-animationEpoch,{glow:glowing,motion:moving});
  if (selected!==null) { const p=diagram.placements.find(p=>p.id===selected);if(p){ctx.beginPath();ctx.arc(panX+p.x*zoom,panY+p.y*zoom,(p.radius+9)*zoom,0,Math.PI*2);ctx.strokeStyle=livingInk.background==='#f0e8d7'?'#73502d':'#f5dfaa';ctx.lineWidth=1.5;ctx.stroke();} }
  const zoomText=`${Math.round(zoom*100)}%`;if($('zoom-label').textContent!==zoomText)$('zoom-label').textContent=zoomText;
}
function fit() { if(!diagram)return;const b=viewport.getBoundingClientRect();zoom=Math.min((b.width-40)/diagram.width,(b.height-45)/diagram.height);panX=(b.width-diagram.width*zoom)/2;panY=(b.height-diagram.height*zoom)/2;draw(); }
function zoomAt(factor,x,y) {const z=Math.min(8,Math.max(.025,zoom*factor));panX=x-(x-panX)*z/zoom;panY=y-(y-panY)*z/zoom;zoom=z;draw();}
viewport.addEventListener('wheel',e=>{e.preventDefault();const b=viewport.getBoundingClientRect();zoomAt(Math.exp(-e.deltaY*.001),e.clientX-b.left,e.clientY-b.top);},{passive:false});
let drag;
viewport.addEventListener('pointerdown',e=>{drag={x:e.clientX,y:e.clientY,px:panX,py:panY,moved:false};viewport.setPointerCapture(e.pointerId);});
viewport.addEventListener('pointermove',e=>{if(!drag)return;const dx=e.clientX-drag.x,dy=e.clientY-drag.y;drag.moved ||= Math.hypot(dx,dy)>4;panX=drag.px+dx;panY=drag.py+dy;draw();});
viewport.addEventListener('pointerup',e=>{if(!drag)return;if(!drag.moved&&diagram){const b=viewport.getBoundingClientRect(),x=(e.clientX-b.left-panX)/zoom,y=(e.clientY-b.top-panY)/zoom;const p=diagram.placements.find(p=>Math.hypot(p.x-x,p.y-y)<p.radius);if(p)inspect(p.id);}drag=null;});
viewport.addEventListener('pointercancel',()=>{drag=null;});
viewport.addEventListener('keydown',e=>{if(['ArrowLeft','ArrowRight','ArrowUp','ArrowDown'].includes(e.key)){e.preventDefault();panX+=e.key==='ArrowLeft'?30:e.key==='ArrowRight'?-30:0;panY+=e.key==='ArrowUp'?30:e.key==='ArrowDown'?-30:0;draw();}if(e.key==='+'||e.key==='=')$('zoom-in').click();if(e.key==='-')$('zoom-out').click();});
let lastViewportWidth = 0, lastViewportHeight = 0;
new ResizeObserver(() => { const bounds = viewport.getBoundingClientRect(); if (bounds.width !== lastViewportWidth || bounds.height !== lastViewportHeight) { lastViewportWidth = bounds.width; lastViewportHeight = bounds.height; fit(); } else { draw(); } }).observe(viewport);
$('fit').onclick=fit;
for(const [id,factor] of [['zoom-in',1.25],['zoom-out',.8]])$(id).onclick=()=>{const b=viewport.getBoundingClientRect();zoomAt(factor,b.width/2,b.height/2);};
function listEntities() {
  const root=$('entities');root.replaceChildren();if(!diagram)return;
  const query=$('search').value.toLowerCase();
  for(const e of diagram.analysis.entities.filter(e=>e.kind!=='merged_impl'&&`${e.qualified_name} ${e.path} ${e.language}`.toLowerCase().includes(query))) {
    const button=document.createElement('button');button.type='button';button.dataset.id=e.id;button.classList.toggle('selected',selected===e.id);
    button.append(document.createTextNode(`${e.kind==='function'?'◎':e.kind==='struct'?'⬡':'◇'} ${e.qualified_name}`));
    const detail=document.createElement('small');detail.textContent=`${e.language} · ${e.path}:${e.line} · ${e.lines} lines`;button.append(detail);button.onclick=()=>{inspect(e.id);const p=diagram.placements.find(p=>p.id===e.id);if(p){const b=viewport.getBoundingClientRect();zoom=Math.max(zoom,1);panX=b.width/2-p.x*zoom;panY=b.height/2-p.y*zoom;draw();}};root.append(button);
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
$('download-svg').onclick=async()=>{try{setBusy(true);download(new Blob([await engine({op:'svg',capsule:capsule(),key:diagramKey})],{type:'image/svg+xml'}),'svg');status('Source-carrying SVG downloaded.');}catch(e){status(e.message,'error');}finally{setBusy(false);}};
$('download-json').onclick=async()=>{
  try{setBusy(true);const data=diagramKey?{version:2,svg:await engine({op:'svg',capsule:capsule(),key:diagramKey})}:{...diagram,snapshot};download(new Blob([JSON.stringify(data)],{type:'application/json'}),'json');status('Offline diagram downloaded. Keyed diagrams require the same key on import.');}
  catch(e){status(e.message,'error');}finally{setBusy(false);}
};
$('download-png').onclick=async()=>{
  try {setBusy(true);status('Encoding the complete repository into lossless PNG pixels…','busy');const bytes=await engine({op:'png',capsule:capsule(),key:diagramKey});download(new Blob([bytes],{type:'image/png'}),'png');status('Recoverable PNG downloaded. Keep the original PNG without resizing or JPEG conversion.');}
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
async function recoverSource(file) {
  if(busy)return;
  try {
    setBusy(true);status('Recovering exact repository bytes from the image…','busy');
    if(file.size>150_000_000)throw new Error('Image exceeds the upload budget.');
    const recovered=await engine({op:'recover',key:$('magic-key').value.trim(),bytes:new Uint8Array(await file.arrayBuffer())});
    const zip=await engine({op:'zip',snapshot:recovered.snapshot});
    snapshot=recovered.snapshot;files=sourceFiles(snapshot);title=snapshot.title;reframe=true;imported=false;
    const next=await transmute(files,title);
    restoreArtwork(next,recovered.svg);
    await install(next);
    download(new Blob([zip],{type:'application/zip'}),'zip');
    status(`Recovered ${snapshot.entries.length} tracked entries with exact bytes. Source ZIP downloaded.`);
  }catch(e){status(e.message,'error');}finally{setBusy(false);}
}
function inputMode() {
  $('input-mark').textContent=inputImage?'◇':'⌘';
  $('generate').textContent=inputImage?'Recover source':'Transmute';
  const arrow=document.createElement('span');arrow.textContent=inputImage?'↓':'↗';$('generate').append(arrow);
  $('input-help').textContent=inputImage?'Image selected. Recover source downloads its exact files as a ZIP. Edit the field to use a GitHub link.':'GitHub link → magic circles · original PNG / SVG → source ZIP. Choose, drop, or paste an image.';
}
function chooseImage(inputs) {
  if(busy)return;
  if(inputs.length!==1){status('Choose one original PNG or SVG image.','error');return;}
  const file=inputs[0];
  if(!/\.(png|svg)$/i.test(file.name)&&!['image/png','image/svg+xml'].includes(file.type)){status('Choose an original exported PNG or SVG image.','error');return;}
  if(file.size>150_000_000){status('Image exceeds the upload budget.','error');return;}
  inputImage=file;$('repo').value=file.name;inputMode();
  status('Magic image ready. Select Recover source to download its source code.');$('generate').focus();
}
$('choose-image').onclick=()=>$('restore').click();
$('restore').onchange=event=>{if(event.target.files.length)chooseImage([...event.target.files]);event.target.value='';};
$('repo').oninput=()=>{inputImage=null;inputMode();};
$('repo').addEventListener('paste',event=>{const files=[...(event.clipboardData?.files||[])];if(files.length){event.preventDefault();chooseImage(files);}});
$('repo-form').addEventListener('dragover',event=>{if(event.dataTransfer?.types.includes('Files')){event.preventDefault();if(!busy)$('repo-form').classList.add('drop-ready');}});
$('repo-form').addEventListener('dragleave',event=>{if(!$('repo-form').contains(event.relatedTarget))$('repo-form').classList.remove('drop-ready');});
$('repo-form').addEventListener('drop',event=>{const files=[...(event.dataTransfer?.files||[])];$('repo-form').classList.remove('drop-ready');if(files.length){event.preventDefault();chooseImage(files);}});
$('decode-names').onclick=async()=>{
  try{const recovered=await engine({op:'names',svg:diagram.svg,key:diagramKey});status(`Decoded ${recovered.length} exact names from contour geometry.`);if(selected!==null){const value=recovered.find(e=>e.entity_id===selected);if(value)$('inspector').querySelector('h3').textContent=value.name;}}
  catch(e){status(e.message,'error');}
};
$('access-button').onclick=()=>{$('access').open=!$('access').open;};
$('repo-form').onsubmit=async e=>{
  e.preventDefault();if(busy)return;
  if(inputImage){await recoverSource(inputImage);return;}
  controller=new AbortController();const activeController=controller;setBusy(true);status('Opening the source grimoire…','busy');
  const token=$('token').value;$('token').value='';
  try {
    const result=await readRepository($('repo').value,{reference:$('reference').value,token,signal:activeController.signal,progress:(message,current,total)=>{if(activeController!==controller)return;status(message,'busy');$('progress').max=total||1;$('progress').value=current;}});
    if(activeController.signal.aborted)return;
    status('Inking syntax and connecting seals…','busy');const next=await transmute(result.files,result.title);next.snapshot=result.snapshot;
    if(activeController.signal.aborted)return;
    snapshot=result.snapshot;files=result.files;title=result.title;imported=false;reframe=true;await install(next);
    status(`Inscribed ${result.files.length} source files; preserved all ${result.snapshot.entries.length} tracked entries at ${result.commit.slice(0,12)}.`);
  } catch(error) {if(!activeController.signal.aborted){activeController.abort();status(error.message,'error');}}
  finally {if(activeController===controller){setBusy(false);controller=null;}}
};
$('cancel').onclick=()=>{controller?.abort();if(worker){worker.terminate();worker=null;for(const p of pending.values())p.reject(new Error('Cancelled'));pending.clear();}status('Transmutation cancelled. The previous inscription remains.');};
async function loadDemo() {if(busy)return;setBusy(true);status('Opening the example grimoire…','busy');try{files=demoFiles;title='Example grimoire · nine language signatures';snapshot=snapshotFromSources(files,title);imported=false;reframe=true;await install(await transmute(files,title));status('Example inscription · try a repository, open a source folder, or inspect a seal.');}catch(e){status(e.message,'error');}finally{setBusy(false);}}
$('demo').onclick=loadDemo;
let debounce;
function rerender(){if(busy)return;if(imported){status('Imported diagrams preserve their original styling. Open source files to change the inscription settings.');return;}clearTimeout(debounce);status('Updating inscription…','busy');debounce=setTimeout(async()=>{if(busy)return;setBusy(true);try{reframe=false;await install(await transmute(files,title));status('Inscription updated.');}catch(e){status(e.message,'error');}finally{setBusy(false);}},150);}
$('min-lines').oninput=()=>{$('threshold-value').textContent=`${$('min-lines').value} lines`;rerender();};$('palette').onchange=rerender;$('wingdings').onchange=rerender;
function colorLegend() {$('language-colors').title='Language hues adapt to the selected background';document.querySelector('.legend').classList.toggle('language-colors',$('language-colors').checked);}
for(const label of document.querySelectorAll('.legend dt[data-language]')){const dot=document.createElement('span');dot.className='language-dot';dot.setAttribute('aria-hidden','true');dot.style.backgroundColor=languageColors[label.dataset.language];label.prepend(dot);}
$('language-colors').onchange=()=>{colorLegend();rerender();};
$('folder').onchange=async event=>{
  if(busy)return;setBusy(true);status('Reading every selected file as exact bytes…','busy');
  try {
    const inputs=[...event.target.files].filter(f=>!/(^|\/)(\.git|target|node_modules|pkg)\//.test(f.webkitRelativePath));
    if(inputs.length>5000||inputs.some(f=>f.size>2_000_000)||inputs.reduce((sum,f)=>sum+f.size,0)>50_000_000)throw new Error('Snapshot budget exceeded: 5,000 files, 2 MB per file, 50 MB total.');
    const entries=[];for(const f of inputs)entries.push({path:f.webkitRelativePath.split('/').slice(1).join('/')||f.name,mode:'100644',data:encodeBytes(new Uint8Array(await f.arrayBuffer())),sha:null});
    title=`Local grimoire · ${inputs[0]?.webkitRelativePath.split('/')[0]||'empty folder'}`;
    snapshot={version:1,title,commit:null,entries};files=sourceFiles(snapshot);imported=false;reframe=true;
    await install(await transmute(files,title));status(`Preserved ${entries.length} selected files exactly. Browser folder mode cannot read executable bits or symlinks; use GitHub or the CLI for those.`);
  }catch(e){status(e.message,'error');}finally{setBusy(false);event.target.value='';}
};
$('json').onchange=async event=>{if(busy)return;const file=event.target.files[0];if(!file)return;setBusy(true);try{if(file.size>50_000_000)throw new Error('Diagram is too large.');let next=JSON.parse(await file.text());if(next.version===2&&typeof next.svg==='string'){
      const recovered=await engine({op:'recover',key:$('magic-key').value.trim(),bytes:new TextEncoder().encode(next.svg)});
      // Work on recovered bytes only after successful key authentication.
      next=await transmute(sourceFiles(recovered.snapshot),recovered.snapshot.title);
      next.snapshot=recovered.snapshot;restoreArtwork(next,recovered.svg);
    }if(!next.analysis?.entities||!Array.isArray(next.placements)||typeof next.svg!=='string'||!Number.isFinite(next.width)||!Number.isFinite(next.height)||next.width<1||next.height<1||next.width>100000||next.height>100000)throw new Error('Choose a JSON diagram exported by Code Is Magic.');const document=new DOMParser().parseFromString(next.svg,'image/svg+xml');if(document.querySelector('parsererror,script,foreignObject,image,use')||document.documentElement.localName!=='svg')throw new Error('Imported SVG contains unsupported elements.');for(const style of document.querySelectorAll('style')){const allowed=/@font-face\s*\{\s*font-family:\s*'CIM(?:Runic|Symbols|Greek)';\s*src:\s*url\('data:font\/ttf;base64,[A-Za-z0-9+/=]+'\)\s*format\('truetype'\);\s*\}/g;if(style.textContent.replace(allowed,'').trim())throw new Error('Imported SVG contains unsupported styling.');}for(const node of document.querySelectorAll('*'))for(const attr of node.attributes)if(/^on/i.test(attr.name)||/href/i.test(attr.name)||/url\(\s*['"]?(?!#)/i.test(attr.value))throw new Error('Imported SVG contains an external resource or executable attribute.');$('language-colors').checked=document.documentElement.getAttribute('data-language-colors')==='true';colorLegend();title=`Imported grimoire · ${file.name}`;snapshot=next.snapshot||null;reframe=true;await install(next);imported=true;status('Offline diagram opened. Original styling preserved.');}catch(e){status(e.message,'error');}finally{setBusy(false);event.target.value='';}};
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
function scheduleAnimation(){
  if(animationFrame!==null||!moving||!image||document.hidden)return;
  const bounds=viewport.getBoundingClientRect();
  previewVisible=bounds.width>0&&bounds.height>0&&bounds.bottom>0&&bounds.top<innerHeight&&bounds.right>0&&bounds.left<innerWidth;
  if(!previewVisible){canvas.dataset.motion='paused';return;}
  canvas.dataset.motion='running';
  animationFrame=requestAnimationFrame(timestamp=>{
    animationFrame=null;
    if(timestamp-lastAnimationFrame>=33){lastAnimationFrame=timestamp;draw(timestamp-animationEpoch);}
    scheduleAnimation();
  });
}
function displayOptions(){
  $('glow-toggle').setAttribute('aria-pressed',String(glowing));
  $('motion-toggle').setAttribute('aria-pressed',String(moving));
  if(!moving&&animationFrame!==null){cancelAnimationFrame(animationFrame);animationFrame=null;}
  canvas.dataset.motion=moving?'running':'paused';draw();scheduleAnimation();
}
$('glow-toggle').onclick=()=>{glowing=!glowing;displayOptions();};
$('motion-toggle').onclick=()=>{moving=!moving;displayOptions();};
document.addEventListener('visibilitychange',()=>{if(document.hidden&&animationFrame!==null){cancelAnimationFrame(animationFrame);animationFrame=null;}else scheduleAnimation();});
window.addEventListener('scroll',scheduleAnimation,{passive:true});
window.addEventListener('resize',scheduleAnimation,{passive:true});
matchMedia('(prefers-reduced-motion: reduce)').addEventListener('change',event=>{if(event.matches){moving=false;displayOptions();}});
$('settings-toggle').onclick=()=>{const open=!document.querySelector('.workbench').hasAttribute('data-settings-open');document.querySelector('.workbench').toggleAttribute('data-settings-open',open);$('settings-toggle').setAttribute('aria-expanded',String(open));};
function theme(mode,changeInk=false){
  document.documentElement.dataset.theme=mode;
  $('theme-label').textContent=mode==='light'?'Dark mode':'Light mode';
  $('theme-toggle').setAttribute('aria-pressed',String(mode==='light'));
  document.querySelector('meta[name="theme-color"]').content=mode==='light'?'#f4f0e5':'#101513';
  try{localStorage.setItem('code-is-magic-theme',mode);}catch{}
  if(changeInk&&!imported){$('palette').value=mode==='light'?'parchment':'gold';rerender();}
}
$('theme-toggle').onclick=()=>theme(document.documentElement.dataset.theme==='light'?'dark':'light',true);
let savedTheme='dark';try{if(localStorage.getItem('code-is-magic-theme')==='light')savedTheme='light';}catch{}
theme(savedTheme);if(savedTheme==='light')$('palette').value='parchment';displayOptions();
loadDemo();
