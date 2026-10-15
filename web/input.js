import { parseRepository } from './repository.js';
export const MAGIC_PATTERN=/\.(png|svg|svgz)$/i;
const MAGIC_TYPES=['image/png','image/svg+xml','application/gzip'];
export const INPUT_HELP='Paste or drop a GitHub link, source files, or a magic PNG / SVG / SVGZ. One input, both directions.';
export function classifyInput(value) {
  if(typeof value==='string') {
    let url;try{url=new URL(value.trim());}catch{throw new Error('Paste an HTTPS GitHub repository or magic image link, or choose source files.');}
    if(url.protocol!=='https:'||url.username||url.password)throw new Error('Use an HTTPS link without embedded credentials.');
    if(url.hostname==='github.com') {parseRepository(url.href);return {kind:'repository',url:url.href,label:url.href};}
    if(!MAGIC_PATTERN.test(url.pathname))throw new Error('Use a GitHub repository link or a direct PNG, SVG, or SVGZ link.');
    return {kind:'image-link',url:url.href,label:url.href};
  }
  const files=[...value];
  if(!files.length)throw new Error('Choose at least one source file or magic image.');
  const images=files.filter(f=>MAGIC_PATTERN.test(f.name)||MAGIC_TYPES.includes(f.type));
  if(images.length===1&&files.length===1&&!files[0].webkitRelativePath&&!files[0].sourcePath?.includes('/')) {
    if(images[0].size>150_000_000)throw new Error('Image exceeds the upload budget.');
    return {kind:'image',file:images[0],label:images[0].name};
  }
  // Whole folders may legitimately contain SVG assets. They remain source entries.
  if(images.length&&files.length>1&&!files.every(f=>f.webkitRelativePath||f.sourcePath))throw new Error('Choose one magic image, or drop source files separately.');
  if(files.length>5000||files.some(f=>f.size>2_000_000)||files.reduce((sum,f)=>sum+f.size,0)>50_000_000)throw new Error('Snapshot budget exceeded: 5,000 files, 2 MB per file, 50 MB total.');
  return {kind:'source',files,label:files.length===1?files[0].name:`${files.length} source files`};
}
export function droppedLink(transfer) {
  const uri=transfer.getData('text/uri-list').split(/\r?\n/).find(line=>line.trim()&&!line.startsWith('#'));
  return (uri||transfer.getData('text/plain')).trim();
}
export async function droppedFiles(transfer) {
  const items=[...(transfer.items||[])];
  const roots=items.filter(i=>i.kind==='file').map(i=>i.webkitGetAsEntry?.());
  if(!roots.some(Boolean))return [...transfer.files];
  const files=[];let bytes=0;
  async function walk(entry) {
    if(/(^|\/)(\.git|target|node_modules|pkg)(\/|$)/.test(entry.fullPath))return;
    if(entry.isFile) {
      const file=await new Promise((resolve,reject)=>entry.file(resolve,reject));
      bytes+=file.size;
      if(file.size>2_000_000&&!(roots.length===1&&entry===roots[0]&&MAGIC_PATTERN.test(file.name)))throw new Error('A source file exceeds 2 MB.');
      if(++count>5000||bytes>150_000_000)throw new Error('Dropped files exceed the upload budget.');
      Object.defineProperty(file,'sourcePath',{value:entry.fullPath.replace(/^\//,'')});files.push(file);
    } else if(entry.isDirectory) {
      const reader=entry.createReader();let batch;
      do {batch=await new Promise((resolve,reject)=>reader.readEntries(resolve,reject));for(const child of batch)await walk(child);}while(batch.length);
    }
  }
  let count=0;for(const entry of roots)if(entry)await walk(entry);
  return files;
}
export async function fetchMagic(url,{signal,fetcher=fetch}={}) {
  // Access tokens never leave readRepository; arbitrary links get no credentials.
  const response=await fetcher(url,{signal,credentials:'omit',referrerPolicy:'no-referrer'});
  if(!response.ok)throw new Error(`Magic image request failed (${response.status}). Download the original and drop it here.`);
  if(Number(response.headers.get('content-length'))>150_000_000)throw new Error('Image exceeds the upload budget.');
  const reader=response.body.getReader(),chunks=[];let size=0;
  try {for(;;){const {done,value}=await reader.read();if(done)break;size+=value.length;if(size>150_000_000)throw new Error('Image exceeds the upload budget.');chunks.push(value);}}
  finally {await reader.cancel();}
  return new File(chunks,decodeURIComponent(new URL(url).pathname.split('/').pop()),{type:response.headers.get('content-type')||''});
}
