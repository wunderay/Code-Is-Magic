import test from 'node:test';
import assert from 'node:assert/strict';
import {sampleCurve,pointOnChannel,createFlow,activeEvent} from '../web/flow.js';
const entities=[{id:0,name:'main',tokens:['zeta','(',')','alpha','(',')']},{id:1,name:'alpha',tokens:[]},{id:2,name:'zeta',tokens:['alpha','(']}];
const placements=entities.map(e=>({id:e.id}));
const channel=(from,to)=>({from,to,...sampleCurve([0,0,15,60,85,-40,100,10])});
test('source order, nested calls, and returns form one connected rehearsal',()=>{
  const paths=[channel(0,1),channel(0,2),channel(2,1)];
  const lanes=createFlow({entities},paths,placements);
  assert.equal(lanes.length,1);
  const events=lanes[0].events.filter(e=>e.kind==='channel');
  assert.deepEqual(events.map(e=>[e.index,e.reverse]),[[1,false],[2,false],[2,true],[1,true],[0,false],[0,true]]);
  assert.deepEqual(createFlow({entities},paths,placements),lanes);
  for(const event of lanes[0].events){assert.equal(activeEvent(lanes[0],event.start+event.duration/2).progress,.5);}
  assert.equal(activeEvent(lanes[0],lanes[0].duration-1),null);
  assert.deepEqual(activeEvent(lanes[0],10),activeEvent(lanes[0],lanes[0].duration+10));
});
test('self recursion and mutual recursion terminate and still return',()=>{
  const paths=[channel(0,0),channel(0,1),channel(1,0)];
  const flow=createFlow({entities},paths,placements);
  const events=flow.flatMap(l=>l.events).filter(e=>e.kind==='channel');
  assert.equal(events.length,6);
  for(let i=0;i<paths.length;i++)assert.deepEqual(events.filter(e=>e.index===i).map(e=>e.reverse),[false,true]);
});
test('large graphs retain every channel; independent and isolated seals stay bounded',()=>{
  const entities=Array.from({length:301},(_,id)=>({id,name:`f${id}`,tokens:[]}));
  const paths=entities.slice(1).map(e=>channel(0,e.id));
  const lanes=createFlow({entities},paths,entities);
  assert.equal(lanes.flatMap(l=>l.events).filter(e=>e.kind==='channel'&&!e.reverse).length,300);
  assert.equal(createFlow({entities},[],entities).length,5);
});
test('arc-length sampling preserves endpoints and has even travel distance',()=>{
  const path=sampleCurve([0,0,0,200,200,200,200,0]);
  assert.deepEqual(pointOnChannel(path,0),{x:0,y:0});
  assert.deepEqual(pointOnChannel(path,1),{x:200,y:0});
  const steps=Array.from({length:11},(_,i)=>pointOnChannel(path,i/10));
  const lengths=steps.slice(1).map((p,i)=>Math.hypot(p.x-steps[i].x,p.y-steps[i].y));
  assert.ok(Math.max(...lengths)/Math.min(...lengths)<1.03);
  const degenerate=sampleCurve(Array(8).fill(0));assert.deepEqual(pointOnChannel(degenerate,.5),{x:0,y:0});
});
test('argument calls run before their enclosing invocation',()=>{
  const entities=[{id:0,name:'main',tokens:['outer','(','inner','(',')',')']},{id:1,name:'outer',tokens:[]},{id:2,name:'inner',tokens:[]}];
  const paths=[channel(0,1),channel(0,2)];
  assert.deepEqual(createFlow({entities},paths,entities)[0].events.filter(e=>e.kind==='channel'&&!e.reverse).map(e=>e.index),[1,0]);
});
test('call illumination stays on its caller and reaches only its destination',async()=>{
  const {illumination}=await import('../web/flow.js');
  const channels=[{from:10,to:20}];
  const start=illumination({kind:'channel',index:0,progress:0,reverse:false},channels);
  assert.deepEqual(start,[{id:10,intensity:.9},{id:20,intensity:0}]);
  const arriving=illumination({kind:'channel',index:0,progress:1,reverse:false},channels);
  assert.equal(arriving[1].id,20);assert.ok(Math.abs(arriving[1].intensity-1)<1e-9);
  const returning=illumination({kind:'channel',index:0,progress:0,reverse:true},channels);
  assert.deepEqual(returning,[{id:20,intensity:.9},{id:10,intensity:0}]);
  assert.equal(illumination({kind:'seal',id:20,progress:.5},channels)[0].intensity,1);
});
