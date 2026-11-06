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

test('embedded magic script unfolds every macro like the Rust expander',async()=>{
 const {pack_magic_svg_keyed}=await import('../web/pkg/code_is_magic.js');
 for(const [key,palette] of [['','gold'],['03'.repeat(32),'parchment']]){
  const input=JSON.stringify(demoFiles);
  const diagram=JSON.parse(transmute_keyed(input,'demo',1,palette,true,true,key));
  const files=demoFiles.map(file=>({path:file.path,mode:'100644',data:Buffer.from(file.content).toString('base64')}));
  const capsule=JSON.stringify({version:1,snapshot:{version:1,title:'demo',commit:null,entries:files},svg:diagram.svg});
  const compact=pack_magic_svg_keyed(capsule,key);
  assert.ok(compact.includes("data-magic-format='3'"));
  const shown=JSON.parse(recover_image_keyed(Buffer.from(compact),key)).svg;
  const script=compact.match(/<script><!\[CDATA\[([\s\S]*?)\]\]><\/script>/)[1];
  const magic=new Function('document',`${script};return MAGIC`)(undefined);
  const glyphs={},collars={};
  for(const [,id,k] of compact.matchAll(/<path id='glyph-(\d+)'[^>]* k='([^']*)'/g))glyphs[id]=k.split(' ').map(Number);
  for(const [,id,language,repeats] of compact.matchAll(/<g id='(L\d+)' l='([^']*)' n='(\d+)'/g))collars[language]=[id,+repeats];
  const macros=[...compact.matchAll(/<([BOINY]) d='([^']*)'\/>/g)];
  const templates={};
  for(const [,id,body] of compact.matchAll(/<g id='(L\d+)'[^>]*>([\s\S]*?)<\/g>/g))templates[id]=body;
  // Outermost parameterized subtrees of the Rust drawing, in document order.
  const subtrees=[];let open=null,depth=0;
  for(const tag of shown.matchAll(/<(\/?)([A-Za-z]+)([^>]*?)(\/?)>/g)){
   const [text,closing,,attributes,selfClosing]=tag;
   if(closing){depth--;if(open&&depth===open.depth){subtrees.push(shown.slice(open.start,tag.index+text.length));open=null;}continue;}
   if(!open&&/ data-m='/.test(attributes)){if(selfClosing){subtrees.push(text);continue;}open={start:tag.index,depth};}
   if(!selfClosing)depth++;
  }
  assert.equal(subtrees.length,macros.length);
  const numbers=markup=>markup.replace(/ (data-[\w-]+|href)='[^']*'/g,'').replace(/#[0-9a-f]{6}/gi,'').match(/-?\d*\.?\d+(?:e-?\d+)?/g)?.map(Number)||[];
  const close=(a,b)=>a.length===b.length&&a.every((v,j)=>Math.abs(v-b[j])<=2e-3+1e-6*Math.abs(v));
  // A collar is three rings plus repeats of its language template. Compare the ring
  // circles, every repeat's translate/rotate/scale, and the template geometry itself.
  const collarMatches=(js,rust,template)=>{
   const circles=markup=>(markup.match(/<circle [^>]*>/g)||[]).slice(0,3).flatMap(numbers);
   const repeats=(markup,pattern)=>[...markup.matchAll(pattern)].map(([,transform])=>numbers(transform));
   const jsRepeats=repeats(js,/<use [^>]*transform='([^']*)'/g),rustRepeats=repeats(rust,/<g transform='([^']*)' stroke-width='.007'>/g);
   const rustTemplate=rust.match(/<g transform='[^']*' stroke-width='.007'>([\s\S]*?)<\/g>/)[1];
   assert.ok(circles(rust).length===12&&rustRepeats.length>0&&numbers(rustTemplate).length>0,'empty collar comparison');
   return close(circles(js),circles(rust))&&numbers(js.match(/stroke-width='[^']*'/)[0]).join()===numbers(rust.match(/stroke-width='[^']*'/)[0]).join()
    &&jsRepeats.length===rustRepeats.length&&jsRepeats.every((r,j)=>close(r,rustRepeats[j]))&&close(numbers(template),numbers(rustTemplate));
  };
  const kinds={};
  macros.forEach(([,kind,payload],i)=>{
   kinds[kind]=(kinds[kind]||0)+1;
   const unfolded=magic[kind](payload,kind==='I'?glyphs:collars);
   if(kind==='O'){
    const [,,,,language]=payload.split(' ');
    assert.equal(unfolded.match(/<use /g).length,collars[language][1]);
    assert.ok(collarMatches(unfolded,subtrees[i],templates[collars[language][0]]),`O macro ${i} geometry`);
    // Mutation controls: a changed radius, rotation or position must be caught.
    const t=payload.split(' ');
    for(const [index,delta] of [[2,0.05],[3,1],[0,0.05]]){
     const mutated=[...t];mutated[index]=String(+mutated[index]+delta);
     assert.ok(!collarMatches(magic.O(mutated.join(' '),collars),subtrees[i],templates[collars[language][0]]),`O mutation ${index} not detected`);
    }
    return;
   }
   const js=numbers(unfolded),rust=numbers(subtrees[i]);
   assert.equal(js.length,rust.length,`${kind} macro ${i} structure`);
   js.forEach((value,j)=>assert.ok(Math.abs(value-rust[j])<=2e-3+1e-6*Math.abs(value),`${kind} macro ${i} number ${j}: ${value} vs ${rust[j]}`));
  });
  for(const kind of 'BOINY')assert.ok(kinds[kind]>0,`no ${kind} macros exercised`);
 }
});
