import test from 'node:test';
import assert from 'node:assert/strict';
import {sampleCurve,createFlow} from '../web/flow.js';
import {writingOrder,writingPlan} from '../web/quill.js';
const entities=[{id:0,name:'main',tokens:['zeta','(',')','alpha','(',')']},{id:1,name:'alpha',tokens:[]},{id:2,name:'zeta',tokens:['alpha','(']},{id:3,name:'lonely',tokens:[]}];
const placements=entities.map((e,i)=>({id:e.id,x:i*100,y:0,radius:20+i*5}));
const channel=(from,to)=>({from,to,...sampleCurve([0,0,15,60,85,-40,100,10])});
test('seals are written in call order, and every seal is written once',()=>{
  const paths=[channel(0,1),channel(0,2),channel(2,1)];
  const lanes=createFlow({entities},paths,placements);
  const order=writingOrder(placements,lanes).map(p=>p.id);
  assert.equal(order[0],0);
  assert.deepEqual([...order].sort(),[0,1,2,3]);
  assert.deepEqual(writingOrder(placements,lanes),writingOrder(placements,lanes));
});
test('braids are drawn after the seals they join, and the plan stays within a pleasant duration',()=>{
  const paths=[channel(0,1),channel(0,2),channel(2,1)];
  const plan=writingPlan(placements,paths,createFlow({entities},paths,placements));
  assert.equal(plan.seals.size,4);assert.equal(plan.strokes.length,3);
  for(const stroke of plan.strokes){
    const {from,to}=paths[stroke.index],a=plan.seals.get(from),b=plan.seals.get(to);
    assert.ok(stroke.start>=a.start+a.duration*.9&&stroke.start>=b.start+b.duration*.75);
  }
  assert.ok(plan.total>1000&&plan.total<4000,`small plans take a few seconds, got ${plan.total}`);
  const many=Array.from({length:400},(_,id)=>({id,x:id%20*50,y:Math.floor(id/20)*50,radius:10+id%7}));
  const big=writingPlan(many,[],[]);
  assert.ok(big.total>=4000&&big.total<=10000,`400 seals take 4-10 s, got ${big.total}`);
  assert.ok(writingPlan(many,[],[],{fast:true}).total<big.total/2);
});
