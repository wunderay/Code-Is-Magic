import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtempSync,writeFileSync,readFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {execFileSync} from 'node:child_process';
import init,{transmute,transmute_with_options,recover_image,pack_png} from '../web/pkg/code_is_magic.js';
import {demoFiles} from '../web/demo.js';
await init({module_or_path:readFileSync(new URL('../web/pkg/code_is_magic_bg.wasm',import.meta.url))});
for(const [wingdings,languageColors,palette] of [[false,false,'gold'],[true,false,'gold'],[false,true,'gold'],[true,true,'parchment']])test(`native/WASM identical: ornaments=${wingdings}, languageColors=${languageColors}, palette=${palette}`,()=>{
 const folder=mkdtempSync(join(tmpdir(),'code-magic-parity-'));
 try{
  for(const file of demoFiles)writeFileSync(join(folder,file.path),file.content);
  const output=join(folder,'output.svg');
  const options=['--palette',palette,...(wingdings?['--wingdings']:[]),...(languageColors?['--language-colors']:[])];
  execFileSync('cargo',['run','--locked','--',folder,'--output',output,...options],{stdio:'pipe'});
  const input=JSON.stringify([...demoFiles].reverse());
  const browser=JSON.parse(languageColors?transmute_with_options(input,folder,12,palette,wingdings,true):transmute(input,folder,12,palette,wingdings));
  const recovered=JSON.parse(recover_image(readFileSync(output)));assert.equal(browser.svg,recovered.svg);assert.equal(recovered.snapshot.entries.length,9);
  assert.equal(browser.analysis.files,9);
  assert.equal(browser.placements.length,28);
  assert.equal(browser.analysis.edges.filter(e=>e.kind==='calls').length,11);
  assert.ok(!browser.svg.includes('<text'));
  const capsule={version:1,snapshot:recovered.snapshot,svg:browser.svg};
  const wasmPng=pack_png(JSON.stringify(capsule));
  const nativePng=join(folder,'source.png');
  // Keep generated outputs outside the captured input folder for this comparison.
  rmSync(output);
  execFileSync('cargo',['run','--locked','--',folder,'--output',nativePng,...options],{stdio:'pipe'});
  assert.deepEqual(Buffer.from(wasmPng),readFileSync(nativePng));
  const roundTrip=JSON.parse(recover_image(wasmPng));
  assert.deepEqual(roundTrip.snapshot,recovered.snapshot);
 }finally{rmSync(folder,{recursive:true,force:true});}
});
