import test from 'node:test';
import assert from 'node:assert/strict';
import {classifyInput,droppedLink,droppedFiles,fetchMagic} from '../web/input.js';
const file=(name,contents='fn main() {}',type='')=>new File([contents],name,{type});
test('one input routes repository links, image links, images, and source files',()=>{
  assert.equal(classifyInput(' https://github.com/wunderay/Code-Is-Magic ').kind,'repository');
  assert.equal(classifyInput('https://example.com/magic.svgz?v=2').kind,'image-link');
  assert.equal(classifyInput([file('magic.svgz')]).kind,'image');
  assert.equal(classifyInput([file('main.rs')]).kind,'source');
  assert.equal(classifyInput([file('main.rs'),file('lib.rs')]).files.length,2);
  for(const value of ['javascript:alert(1)','http://example.com/a.svg','https://user:pass@example.com/a.svg','https://github.com/user/repo/tree/main','https://example.com/a.js'])assert.throws(()=>classifyInput(value));
  assert.throws(()=>classifyInput([]));
});
test('invalid selections cannot silently drop files or exceed source budgets',()=>{
  assert.throws(()=>classifyInput([file('a.svg'),file('main.rs')]));
  assert.throws(()=>classifyInput([{name:'main.rs',size:2_000_001}]));
  assert.throws(()=>classifyInput([{name:'magic.svgz',size:150_000_001}]));
  const inputs=[file('icon.svg'),file('main.rs')];
  inputs.forEach(f=>Object.defineProperty(f,'webkitRelativePath',{value:`source/${f.name}`}));
  assert.equal(classifyInput(inputs).kind,'source');
});
test('dropped URI lists and plain URLs use the same link classifier',()=>{
  for(const data of [{'text/uri-list':'# comment\r\nhttps://github.com/wunderay/Code-Is-Magic\r\n'}, {'text/plain':'https://github.com/wunderay/Code-Is-Magic'}]){
    assert.equal(classifyInput(droppedLink({getData:type=>data[type]||''})).kind,'repository');
  }
});
test('directory drop drains every reader batch and excludes build directories',async()=>{
  const entry=name=>({kind:'file',isFile:true,fullPath:`/source/${name}`,file:resolve=>resolve(file(name))});
  const batches=[[entry('main.rs')],[entry('lib.rs'),entry('target/output.rs')],[]];
  const root={isDirectory:true,fullPath:'/source',createReader:()=>({readEntries:resolve=>resolve(batches.shift())})};
  const files=await droppedFiles({items:[{kind:'file',webkitGetAsEntry:()=>root}]});
  assert.deepEqual(files.map(f=>f.sourcePath),['source/main.rs','source/lib.rs']);
});
test('direct image links omit credentials and enforce download bounds',async()=>{
  let options;
  const fetcher=async(url,opts)=>{options=opts;return new Response('magic',{headers:{'content-type':'application/gzip'}});};
  assert.equal((await fetchMagic('https://example.com/a.svgz',{fetcher})).name,'a.svgz');
  assert.equal(options.credentials,'omit');assert.equal(options.headers,undefined);
  await assert.rejects(fetchMagic('https://example.com/a.svgz',{fetcher:async()=>new Response('',{headers:{'content-length':'150000001'}})}),/budget/);
});
