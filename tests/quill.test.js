import test from 'node:test';
import assert from 'node:assert/strict';
import {sampleCurve,createFlow} from '../web/flow.js';
import {writingOrder,writingPlan,unrollingReach} from '../web/quill.js';
const entities=[{id:0,name:'main',tokens:['zeta','(',')','alpha','(',')']},{id:1,name:'alpha',tokens:[]},{id:2,name:'zeta',tokens:['alpha','(']},{id:3,name:'lonely',tokens:[]}];
const placements=entities.map((spell,nthSpell)=>({id:spell.id,x:nthSpell*100,y:0,radius:20+nthSpell*5}));
const channel=(from,to)=>({from,to,...sampleCurve([0,0,15,60,85,-40,100,10])});
test('seals are written in call order, and every seal is written once',()=>{
  const paths=[channel(0,1),channel(0,2),channel(2,1)];
  const lanes=createFlow({entities},paths,placements);
  const procession=writingOrder(placements,lanes).map(seal=>seal.id);
  assert.equal(procession[0],0);
  assert.deepEqual([...procession].sort(),[0,1,2,3]);
  assert.deepEqual(writingOrder(placements,lanes),writingOrder(placements,lanes));
});
test('braids are drawn after the seals they join, and the plan stays within a pleasant duration',()=>{
  const paths=[channel(0,1),channel(0,2),channel(2,1)];
  const plan=writingPlan(placements,paths,createFlow({entities},paths,placements));
  assert.equal(plan.seals.size,4);assert.equal(plan.strokes.length,3);
  for(const stroke of plan.strokes){
    const {from,to}=paths[stroke.index],caller=plan.seals.get(from),callee=plan.seals.get(to);
    assert.ok(stroke.start>=caller.start+caller.duration*.9&&stroke.start>=callee.start+callee.duration*.75);
  }
  assert.ok(plan.total>1000&&plan.total<4000,`small plans take a few seconds, got ${plan.total}`);
  const multitude=Array.from({length:400},(_unwritten,sigil)=>({id:sigil,x:sigil%20*50,y:Math.floor(sigil/20)*50,radius:10+sigil%7}));
  const grandPlan=writingPlan(multitude,[],[]);
  assert.ok(grandPlan.total>=4000&&grandPlan.total<=10000,`400 seals take 4-10 s, got ${grandPlan.total}`);
  assert.ok(writingPlan(multitude,[],[],{fast:true}).total<grandPlan.total/2);
});
test('on a scroll, seals wait for the parchment to unroll past them, from its heart to its rods',()=>{
  const scroll={axis:'across',heart:{x:150,y:0},halfReach:200};
  const plan=writingPlan(placements,[],[],{scroll});
  const byReach=[...placements].sort((nearer,farther)=>unrollingReach(nearer,scroll)-unrollingReach(farther,scroll));
  for(let nthSeal=1;nthSeal<byReach.length;nthSeal++)assert.ok(plan.seals.get(byReach[nthSeal].id).start>=plan.seals.get(byReach[nthSeal-1].id).start);
  for(const seal of placements)assert.ok(Math.abs(plan.seals.get(seal.id).start-plan.span*unrollingReach(seal,scroll))<1e-9);
  assert.equal(unrollingReach({x:150,y:0,radius:0},scroll),0);
  assert.equal(unrollingReach({x:9999,y:0,radius:10},scroll),1);
  assert.equal(unrollingReach({x:150,y:120,radius:0},{...scroll,axis:'down'}),.6);
});
