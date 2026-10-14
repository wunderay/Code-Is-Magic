import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtempSync,writeFileSync,readFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {execFileSync} from 'node:child_process';
import init,{transmute} from '../web/pkg/code_is_magic.js';
import {demoFiles} from '../web/demo.js';
await init({module_or_path:readFileSync(new URL('../web/pkg/code_is_magic_bg.wasm',import.meta.url))});
for(const wingdings of [false,true])test(`native and WASM produce the same complete diagram (Wingdings=${wingdings})`,()=>{
 const folder=mkdtempSync(join(tmpdir(),'code-magic-parity-'));
 try{
  for(const file of demoFiles)writeFileSync(join(folder,file.path),file.content);
  const output=join(folder,'output.svg');
  execFileSync('cargo',['run','--locked','--',folder,'--output',output,...(wingdings?['--wingdings']:[])],{stdio:'pipe'});
  const browser=JSON.parse(transmute(JSON.stringify([...demoFiles].reverse()),folder,12,'gold',wingdings));
  assert.equal(browser.svg,readFileSync(output,'utf8'));
  assert.equal(browser.analysis.files,9);
  assert.equal(browser.placements.length,28);
  assert.equal(browser.analysis.edges.filter(e=>e.kind==='calls').length,11);
  assert.ok(browser.svg.includes("data:font/ttf;base64,"));
 }finally{rmSync(folder,{recursive:true,force:true});}
});
