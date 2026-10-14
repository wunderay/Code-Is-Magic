import { sourceExtensions } from './languages.js';
export const SOURCE_PATTERN = new RegExp(`\\.(${sourceExtensions.join('|')})$`);
export function parseRepository(input) {
  let url; try { url = new URL(input); } catch { throw new Error('Enter a repository URL such as https://github.com/owner/repo.'); }
  if (url.protocol !== 'https:' || url.hostname !== 'github.com' || url.username || url.password || url.search || url.hash) throw new Error('Use an HTTPS github.com repository URL without a query or fragment.');
  const parts = url.pathname.replace(/\/$/, '').replace(/\.git$/, '').split('/').filter(Boolean);
  if (parts.length !== 2 || parts.some(p => !/^[A-Za-z0-9_.-]+$/.test(p) || p === '.' || p === '..')) throw new Error('Use the repository root URL. Choose a branch or commit under Revision & access.');
  return parts.join('/');
}
export async function readRepository(input, { reference = '', token = '', signal, progress = () => {}, fetcher = fetch } = {}) {
  const slug = parseRepository(input);
  const api = async path => {
    const response = await fetcher(`https://api.github.com/${path}`, { signal, headers: { Accept: 'application/vnd.github+json', ...(token ? { Authorization: `Bearer ${token}` } : {}) } });
    if (!response.ok) {
      if (response.status === 403 || response.status === 429) throw new Error('GitHub rate limit or access restriction. Supply a read-only token under Revision & access, wait for the limit to reset, or open a local source folder.');
      if (response.status === 404) throw new Error('Repository or revision not found. A private repository needs a token with read access.');
      throw new Error(`GitHub request failed (${response.status}); no partial diagram was created.`);
    }
    return response.json();
  };
  progress('Resolving a fixed source revision…', 0, 1);
  const meta = await api(`repos/${slug}`);
  const commit = await api(`repos/${slug}/commits/${encodeURIComponent(reference || meta.default_branch)}`);
  const root = commit.commit.tree.sha;
  let tree = await api(`repos/${slug}/git/trees/${root}?recursive=1`);
  if (tree.truncated) {
    const collected = [];
    async function walk(sha, prefix = '') {
      const result = await api(`repos/${slug}/git/trees/${sha}`);
      if (result.truncated) throw new Error('GitHub truncated even a single directory. Use a local source folder.');
      for (const entry of result.tree) {
        if (entry.type === 'tree') await walk(entry.sha, `${prefix}${entry.path}/`);
        else collected.push({ ...entry, path: prefix + entry.path });
      }
    }
    await walk(root); tree = { tree: collected };
  }
  const blobs = tree.tree.filter(e => e.type === 'blob').sort((a,b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0);
  if (blobs.length > 5000) throw new Error('More than 5,000 tracked files. Select a smaller repository.');
  if (blobs.some(e => e.size > 2_000_000) || blobs.reduce((sum,e) => sum + (e.size || 0), 0) > 50_000_000) throw new Error('Source budget exceeded: 2 MB per file / 50 MB total. No partial diagram was created.');
  let cursor = 0, completed = 0, bytes = 0;
  const entries = new Array(blobs.length);
  // Every file is fetched by immutable blob SHA. Do not emit a partial result on any failure.
  await Promise.all(Array.from({ length: Math.min(4, blobs.length) }, async () => {
    while (cursor < blobs.length) {
      const index = cursor++, entry = blobs[index];
      const blob = await api(`repos/${slug}/git/blobs/${entry.sha}`);
      if (blob.encoding !== 'base64') throw new Error(`Unsupported encoding: ${entry.path}`);
      const raw = atob(blob.content.replace(/\s/g, ''));
      const buffer = Uint8Array.from(raw, c => c.charCodeAt(0));
      bytes += buffer.length;
      if (buffer.length > 2_000_000 || bytes > 50_000_000) throw new Error('Downloaded source exceeds the size budget.');
      entries[index] = { path: entry.path, mode: entry.mode || '100644', data: blob.content.replace(/\s/g,''), sha: entry.sha };
      progress(`Reading ${++completed}/${blobs.length}: ${entry.path}`, completed, blobs.length);
    }
  }));
  const submodules = tree.tree.filter(e=>e.type==='commit');
  for(const entry of submodules) entries.push({path:entry.path,mode:'160000',data:'',sha:entry.sha});
  if(entries.length>5000)throw new Error('Snapshot exceeds 5,000 tracked entries.');
  entries.sort((a,b)=>a.path<b.path?-1:a.path>b.path?1:0);
  const title=`${slug} @ ${commit.sha.slice(0,12)}`;
  const snapshot={version:1,title,commit:commit.sha,entries};
  return { files: sourceFiles(snapshot), snapshot, title, commit:commit.sha, skipped:0, submodules:submodules.length };
}
export function sourceFiles(snapshot) {
  return snapshot.entries.filter(e=>e.mode.startsWith('100')&&SOURCE_PATTERN.test(e.path)).flatMap(e=>{
    try {const bytes=Uint8Array.from(atob(e.data),c=>c.charCodeAt(0));return [{path:e.path,content:new TextDecoder('utf-8',{fatal:true}).decode(bytes)}];} catch{return [];}
  });
}
export function encodeBytes(bytes) {
  let binary='';for(let i=0;i<bytes.length;i+=32768)binary+=String.fromCharCode(...bytes.subarray(i,i+32768));return btoa(binary);
}
export function snapshotFromSources(files,title) {
  return {version:1,title,commit:null,entries:files.map(f=>({path:f.path,mode:'100644',data:encodeBytes(new TextEncoder().encode(f.content)),sha:null}))};
}
