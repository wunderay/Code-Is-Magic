// Ship an immutable, complete project snapshot. No GitHub quota on first visit.
import { execFileSync } from 'node:child_process';
import { writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
const root=fileURLToPath(new URL('../',import.meta.url));
const git=(...args)=>execFileSync('git',args,{cwd:root,maxBuffer:55_000_000});
const commit=git('rev-parse','HEAD').toString().trim();
const entries=git('ls-tree','-rz',commit).toString().split('\0').filter(Boolean).map(line=>{
  const [,mode,type,sha,path]=line.match(/^(\d+) (\w+) ([0-9a-f]+)\t([\s\S]+)$/);
  return {path,mode,sha,data:type==='blob'?git('cat-file','blob',sha).toString('base64'):''};
});
writeFileSync(new URL('../web/default-project.json',import.meta.url),JSON.stringify({version:1,title:`Code is Magic @ ${commit.slice(0,12)}`,commit,entries}));
console.log(`Bundled Code is Magic: ${entries.length} entries at ${commit.slice(0,12)}`);
