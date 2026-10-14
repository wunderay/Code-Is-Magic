import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtempSync,writeFileSync,readFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {execFileSync} from 'node:child_process';
import init,{transmute,transmute_with_options,recover_image} from '../web/pkg/code_is_magic.js';
import {demoFiles} from '../web/demo.js';
await init({module_or_path:readFileSync(new URL('../web/pkg/code_is_magic_bg.wasm',import.meta.url))});
const {transmute_keyed,pack_magic_keyed,pack_png_keyed,optimize_png,recover_image_keyed,recover_names_keyed}=await import('../web/pkg/code_is_magic.js');

for(const [wingdings,languageColors,palette] of [[false,false,'gold'],[true,false,'gold'],[false,true,'gold'],[true,true,'parchment']])test(`canonical native/WASM parity: ornaments=${wingdings}, languageColors=${languageColors}, palette=${palette}`,()=>{
 const folder=mkdtempSync(join(tmpdir(),'code-magic-parity-'));
 try{
  for(const file of demoFiles)writeFileSync(join(folder,file.path),file.content);
  const output=join(folder,'output.svgz');
  const options=['--palette',palette,...(wingdings?['--wingdings']:[]),...(languageColors?['--language-colors']:[])];
  execFileSync('cargo',['run','--locked','--',folder,'--output',output,...options],{stdio:'pipe'});
  const input=JSON.stringify([...demoFiles].reverse());
  const browser=JSON.parse(languageColors?transmute_with_options(input,folder,12,palette,wingdings,true):transmute(input,folder,12,palette,wingdings));
  const recovered=JSON.parse(recover_image(readFileSync(output)));
  assert.equal(recovered.snapshot.entries.length,9);
  assert.equal(browser.analysis.files,9);
  assert.equal(browser.placements.length,28);
  assert.equal(browser.analysis.edges.filter(e=>e.kind==='calls').length,11);
  assert.ok(!browser.svg.includes('<text'));
  const canonical=pack_magic_keyed(JSON.stringify({version:1,snapshot:recovered.snapshot,svg:browser.svg}),'');
  assert.ok(Buffer.from(canonical).equals(readFileSync(output)),'Native/WASM canonical SVGZ bytes differ');
  assert.deepEqual(JSON.parse(recover_image(canonical)).snapshot,recovered.snapshot);
  const forbidden=join(folder,'forbidden.png');
  assert.throws(()=>execFileSync('cargo',['run','--locked','--',folder,'--output',forbidden,...options],{stdio:'pipe'}));
 }finally{rmSync(folder,{recursive:true,force:true});}
});

for(const palette of ['gold','parchment'])test(`keyed canonical parity and authenticated recovery: ${palette}`,()=>{
 const folder=mkdtempSync(join(tmpdir(),'code-magic-keyed-'));
 const key='01'.repeat(32),wrong='02'.repeat(32);
 try{
  const source=join(folder,'source');execFileSync('mkdir',[source]);
  for(const file of demoFiles)writeFileSync(join(source,file.path),file.content);
  const env={...process.env,CODE_IS_MAGIC_KEY:key};
  const magicPath=join(folder,'out.svgz');
  const options=['--palette',palette,'--language-colors','--wingdings'];
  execFileSync('cargo',['run','--locked','--',source,'--output',magicPath,...options],{env,stdio:'pipe'});
  const exported=readFileSync(magicPath);
  assert.throws(()=>recover_image(exported),/requires.*key/i);
  assert.throws(()=>recover_image_keyed(exported,wrong),/Wrong magic key/);
  const recovered=JSON.parse(recover_image_keyed(exported,key));
  const browser=JSON.parse(transmute_keyed(JSON.stringify([...demoFiles].reverse()),source,12,palette,true,true,key));
  const canonical=pack_magic_keyed(JSON.stringify({version:1,snapshot:recovered.snapshot,svg:browser.svg}),key);
  assert.ok(Buffer.from(canonical).equals(exported),'Keyed native/WASM canonical bytes differ');
  assert.deepEqual(JSON.parse(recover_image_keyed(canonical,key)).snapshot,recovered.snapshot);
  assert.throws(()=>recover_image_keyed(canonical,wrong),/Wrong magic key/);
  assert.equal(JSON.parse(recover_names_keyed(recovered.svg,key)).length,28);
  assert.deepEqual(JSON.parse(recover_names_keyed(recovered.svg,key)),JSON.parse(recover_names_keyed(browser.svg,key)));

  // Legacy artifacts remain readable/optimizable for migration, but the CLI no longer creates them.
  const legacyPng=pack_png_keyed(JSON.stringify({version:1,snapshot:recovered.snapshot,svg:browser.svg}),key);
  assert.deepEqual(JSON.parse(recover_image_keyed(legacyPng,key)).snapshot,recovered.snapshot);
  const legacyPath=join(folder,'legacy.png'),optimizedPath=join(folder,'optimized.png');
  writeFileSync(legacyPath,legacyPng);
  execFileSync('cargo',['run','--locked','--','--optimize',legacyPath,'--output',optimizedPath],{stdio:'pipe'});
  const smaller=optimize_png(legacyPng);
  assert.ok(Buffer.from(smaller).equals(readFileSync(optimizedPath)),'Native/WASM legacy PNG optimization differs');
  assert.ok(smaller.length<=legacyPng.length);
  assert.deepEqual(JSON.parse(recover_image_keyed(smaller,key)).snapshot,recovered.snapshot);
 }finally{rmSync(folder,{recursive:true,force:true});}
});
