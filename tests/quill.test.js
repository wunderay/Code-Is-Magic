import test from 'node:test';
import assert from 'node:assert/strict';
import {sampleCurve,createFlow} from '../web/flow.js';
import {writingOrder,writingPlan,unrollingReach,screenReach,unrollingOpening,glide,followingView,Quill,quillCadence} from '../web/quill.js';
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
test('an unstarted quill below its sight threshold requests no frames at all',()=>{
  const waitingQuill={begin:Infinity,fast:false};
  // The review's probe: a 650px scroll at top=700 in an 800px window shows 100px, under 40% of 650px.
  const glimpse=Math.min(700+650,800)-Math.max(700,0),fullView=Math.min(650,800);
  let framesRequested=0;
  for(let nthFrame=0;nthFrame<600;nthFrame++)if(quillCadence(waitingQuill,glimpse,{fullView})!=='rest')framesRequested++;
  assert.equal(framesRequested,0);
  assert.equal(quillCadence(waitingQuill,300,{fullView}),'begin');
  assert.equal(quillCadence({begin:Infinity,fast:true},40,{fullView,fast:true}),'begin');
  assert.equal(quillCadence({begin:12,fast:false},0,{fullView}),'write');
  assert.equal(quillCadence(null,0,{fullView}),'idle');
});
test('on the screen, the opening must bare a seal where the current pan and zoom put it',()=>{
  const view={axis:'across',panX:0,panY:0,zoom:1,width:400,height:300};
  assert.equal(screenReach({x:200,y:0,radius:0},view),0);
  assert.equal(screenReach({x:300,y:0,radius:50},view),(100+50*1.22)/200);
  // Zoomed in and panned, the same seal lies far from the heart and needs the whole scroll.
  assert.equal(screenReach({x:300,y:0,radius:50},{...view,zoom:3,panX:-400}),1);
  assert.equal(screenReach({x:0,y:240,radius:0},{...view,axis:'down'}),90/150);
});
test('the opening keeps the even pace, widens for the pen, never closes and ends open',()=>{
  assert.equal(unrollingOpening(.3,[]),.3);
  assert.equal(unrollingOpening(.3,[.2,.7]),.7);
  assert.equal(unrollingOpening(.3,[.2],.5),.5);
  assert.equal(unrollingOpening(1.4,[]),1);
  const seal={id:0,x:100,y:0,radius:20},inkwell={bounds:{left:-100,right:100,top:-50,bottom:50},paths:[],flow:[]};
  const quill=new Quill({placements:[seal]},inkwell,{now:0});
  const view={panX:200,panY:150,zoom:1,width:400,height:300};
  let opening=0;
  for(let hourglass=0;hourglass<=quill.plan.total+10;hourglass+=25){
    const wider=quill.unrolled(hourglass,view);
    assert.ok(wider>=opening);opening=wider;
  }
  assert.equal(opening,1);
  // Zoomed in on the seal's far side, the pen's seal is bared before the even pace reaches it.
  const central=new Quill({placements:[{...seal,x:0}]},inkwell,{now:0}),writingAt=central.plan.seals.get(0).start;
  const evenPace=new Quill({placements:[{...seal,x:0}]},inkwell,{now:0}).unrolled(writingAt);
  assert.ok(evenPace<1);
  assert.equal(central.unrolled(writingAt,{...view,zoom:4,panX:-600}),1);
});
test('the view glides to its target without overshoot, whatever the frame rate',()=>{
  const settle=frames=>{let state={value:0,velocity:0};for(let nthFrame=0;nthFrame<frames;nthFrame++){state=glide(state.value,state.velocity,10,2/frames,.8);assert.ok(state.value<=10+1e-9);}return state.value;};
  assert.ok(Math.abs(settle(60)-settle(240))<.05);
  assert.ok(settle(60)>9);
  assert.equal(glide(3,0,10,0,.8).value,3);
});
test('while writing, the view draws nearer by at most a few times the fitted zoom, and toward the seal',()=>{
  const fitted={zoom:.1,panX:200,panY:150},room={width:400,height:300};
  const small=followingView({x:1000,y:500,radius:5},fitted,room);
  assert.ok(Math.abs(small.zoom-.35)<1e-9);
  const fittedX=fitted.panX+1000*fitted.zoom,followedX=small.panX+1000*small.zoom;
  assert.ok(Math.abs(followedX-200)<Math.abs(fittedX-200));
  const grand=followingView({x:0,y:0,radius:5000},fitted,room);
  assert.deepEqual(grand,{zoom:.1,panX:200,panY:150});
});
