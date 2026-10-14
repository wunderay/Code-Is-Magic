import { readRepository, SOURCE_PATTERN } from './repository.js';
import { demoFiles } from './demo.js';
const $ = id => document.getElementById(id);
const canvas = $('canvas'), ctx = canvas.getContext('2d'), viewport = $('viewport');
let worker, requestId = 0, pending = new Map(), image, diagram, files = demoFiles, title = 'Example grimoire · nine language signatures';
let zoom = 1, panX = 0, panY = 0, selected = null, controller = null, busy = false, imported = false, reframe = true;
function status(message, type = '') { $('status').textContent = message; $('status').className = `status ${type}`; }
function setBusy(value) { busy = value; $('generate').disabled = value; $('demo').disabled = value; $('cancel').hidden = !value; $('progress').hidden = !value; }
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
async function transmute(inputFiles, inputTitle) {
  const w = getWorker(), id = ++requestId;
  return new Promise((resolve,reject) => { pending.set(id,{resolve,reject}); w.postMessage({id,files:inputFiles,title:inputTitle,minLines:Number($('min-lines').value),palette:$('palette').value,wingdings:$('wingdings').checked}); });
}
async function install(next) {
  // Generated SVG has no external images or executable source text.
  const blob = new Blob([next.svg], {type:'image/svg+xml'}), url = URL.createObjectURL(blob);
  const nextImage = new Image();
  try { await new Promise((resolve,reject) => { nextImage.onload = resolve; nextImage.onerror = () => reject(new Error('The inscription image could not render.')); nextImage.src = url; }); }
  finally { URL.revokeObjectURL(url); }
  diagram = next; image = nextImage; selected = null;
  if (reframe) fit(); else draw();
  $('diagram-title').textContent = title;
  const visible = next.analysis.entities.filter(e=>e.kind!=='merged_impl');
  const calls = next.analysis.edges.filter(e=>e.kind==='calls').length;
  $('canvas-stats').textContent = `${visible.length} SEALS · ${calls} CALL CHANNELS`;
  $('coverage').textContent = `${next.analysis.files} source files · ${next.analysis.unresolved_calls} unresolved / external calls`;
  canvas.setAttribute('aria-label', `${visible.length} magic seals with ${calls} call channels for ${title}`);
  for (const id of ['download-png','download-svg','download-json']) $(id).disabled = false;
  listEntities(); inspect(null);
}
function draw() {
  const bounds = viewport.getBoundingClientRect(), dpr = Math.min(devicePixelRatio || 1, 2);
  const w = Math.round(bounds.width*dpr), h = Math.round(bounds.height*dpr);
  if (canvas.width!==w || canvas.height!==h) { canvas.width=w;canvas.height=h; }
  ctx.setTransform(dpr,0,0,dpr,0,0);ctx.clearRect(0,0,bounds.width,bounds.height);
  if (!image) return;
  ctx.fillStyle = $('palette').value==='parchment' ? '#f0e8d7' : $('palette').value==='astral' ? '#090d20' : '#0c1112';
  ctx.fillRect(0,0,bounds.width,bounds.height);ctx.drawImage(image,panX,panY,diagram.width*zoom,diagram.height*zoom);
  if (selected!==null) { const p=diagram.placements.find(p=>p.id===selected);if(p){ctx.beginPath();ctx.arc(panX+p.x*zoom,panY+p.y*zoom,(p.radius+9)*zoom,0,Math.PI*2);ctx.strokeStyle='#f5dfaa';ctx.lineWidth=1.5;ctx.stroke();} }
  $('zoom-label').textContent = `${Math.round(zoom*100)}%`;
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
$('download-svg').onclick=()=>download(new Blob([diagram.svg],{type:'image/svg+xml'}),'svg');
$('download-json').onclick=()=>download(new Blob([JSON.stringify(diagram)],{type:'application/json'}),'json');
$('download-png').onclick=async()=>{
  try {const exportCanvas=document.createElement('canvas');const scale=Math.min(2,8192/Math.max(diagram.width,diagram.height),Math.sqrt(32_000_000/(diagram.width*diagram.height)));exportCanvas.width=Math.round(diagram.width*scale);exportCanvas.height=Math.round(diagram.height*scale);exportCanvas.getContext('2d').drawImage(image,0,0,exportCanvas.width,exportCanvas.height);const blob=await new Promise(resolve=>exportCanvas.toBlob(resolve,'image/png'));if(!blob)throw new Error('PNG export failed; download the resolution-independent SVG.');download(blob,'png');status(`PNG downloaded · ${exportCanvas.width} × ${exportCanvas.height} pixels.`);}
  catch(e){status(e.message,'error');}
};
$('access-button').onclick=()=>{$('access').open=!$('access').open;};
$('repo-form').onsubmit=async e=>{
  e.preventDefault();if(busy)return;
  controller=new AbortController();const activeController=controller;setBusy(true);status('Opening the source grimoire…','busy');
  const token=$('token').value;$('token').value='';
  try {
    const result=await readRepository($('repo').value,{reference:$('reference').value,token,signal:activeController.signal,progress:(message,current,total)=>{if(activeController!==controller)return;status(message,'busy');$('progress').max=total||1;$('progress').value=current;}});
    if(activeController.signal.aborted)return;
    status('Inking syntax and connecting seals…','busy');const next=await transmute(result.files,result.title);
    if(activeController.signal.aborted)return;
    files=result.files;title=result.title;imported=false;reframe=true;await install(next);
    status(`Inscribed ${result.files.length} source files at ${result.commit.slice(0,12)}. ${result.skipped} non-source files and ${result.submodules} submodules excluded.`);
  } catch(error) {if(!activeController.signal.aborted){activeController.abort();status(error.message,'error');}}
  finally {if(activeController===controller){setBusy(false);controller=null;}}
};
$('cancel').onclick=()=>{controller?.abort();if(worker){worker.terminate();worker=null;for(const p of pending.values())p.reject(new Error('Cancelled'));pending.clear();}status('Transmutation cancelled. The previous inscription remains.');};
async function loadDemo() {if(busy)return;setBusy(true);status('Opening the example grimoire…','busy');try{files=demoFiles;title='Example grimoire · nine language signatures';imported=false;reframe=true;await install(await transmute(files,title));status('Example inscription · try a repository, open a source folder, or inspect a seal.');}catch(e){status(e.message,'error');}finally{setBusy(false);}}
$('demo').onclick=loadDemo;
let debounce;
function rerender(){if(imported){status('Imported diagrams preserve their original styling. Open source files to change the inscription settings.');return;}clearTimeout(debounce);status('Updating inscription…','busy');debounce=setTimeout(async()=>{if(busy)return;setBusy(true);try{reframe=false;await install(await transmute(files,title));status('Inscription updated.');}catch(e){status(e.message,'error');}finally{setBusy(false);}},150);}
$('min-lines').oninput=()=>{$('threshold-value').textContent=`${$('min-lines').value} lines`;rerender();};$('palette').onchange=rerender;$('wingdings').onchange=()=>{
  if($('wingdings').checked){
    const test=document.createElement('canvas').getContext('2d');
    test.font='24px monospace';const baseline=test.measureText('JKNYablnpvzu').width;
    test.font='24px Wingdings, monospace';
    if(Math.abs(test.measureText('JKNYablnpvzu').width-baseline)<.01){$('wingdings').checked=false;status('Wingdings is not installed in this browser environment. Portable runes and dingbats remain available.');return;}
  }
  rerender();
};
$('folder').onchange=async event=>{if(busy)return;setBusy(true);status('Reading local source files…','busy');try{const inputs=[...event.target.files].filter(f=>SOURCE_PATTERN.test(f.name)&&!/(^|\/)(\.git|target|node_modules)\//.test(f.webkitRelativePath));if(inputs.length>5000||inputs.some(f=>f.size>2_000_000)||inputs.reduce((sum,f)=>sum+f.size,0)>50_000_000)throw new Error('Source budget exceeded: 5,000 files, 2 MB per file, 50 MB total.');const local=[];for(const f of inputs)local.push({path:f.webkitRelativePath||f.name,content:new TextDecoder('utf-8',{fatal:true}).decode(await f.arrayBuffer())});const localTitle=`Local grimoire · ${inputs[0]?.webkitRelativePath.split('/')[0]||'empty folder'}`;const next=await transmute(local,localTitle);files=local;title=localTitle;imported=false;reframe=true;await install(next);status(`Inscribed ${local.length} local source files. No source code was uploaded.`);}catch(e){status(e.message,'error');}finally{setBusy(false);event.target.value='';}};
$('json').onchange=async event=>{if(busy)return;const file=event.target.files[0];if(!file)return;try{if(file.size>50_000_000)throw new Error('Diagram is too large.');const next=JSON.parse(await file.text());if(!next.analysis?.entities||!Array.isArray(next.placements)||typeof next.svg!=='string'||!Number.isFinite(next.width)||!Number.isFinite(next.height)||next.width<1||next.height<1||next.width>100000||next.height>100000)throw new Error('Choose a JSON diagram exported by Code Is Magic.');const document=new DOMParser().parseFromString(next.svg,'image/svg+xml');if(document.querySelector('parsererror,script,foreignObject,image,use')||document.documentElement.localName!=='svg')throw new Error('Imported SVG contains unsupported elements.');for(const style of document.querySelectorAll('style')){const allowed=/@font-face\s*\{\s*font-family:\s*'CIM(?:Runic|Symbols|Greek)';\s*src:\s*url\('data:font\/ttf;base64,[A-Za-z0-9+/=]+'\)\s*format\('truetype'\);\s*\}/g;if(style.textContent.replace(allowed,'').trim())throw new Error('Imported SVG contains unsupported styling.');}for(const node of document.querySelectorAll('*'))for(const attr of node.attributes)if(/^on/i.test(attr.name)||/href/i.test(attr.name)||/url\(\s*['"]?(?!#)/i.test(attr.value))throw new Error('Imported SVG contains an external resource or executable attribute.');title=`Imported grimoire · ${file.name}`;reframe=true;await install(next);imported=true;status('Offline diagram opened. Original styling preserved.');}catch(e){status(e.message,'error');}finally{event.target.value='';}};
loadDemo();
