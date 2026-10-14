import init, { transmute, pack_png, pack_svg, recover_image, recover_zip, recover_names } from './pkg/code_is_magic.js';
const ready=init();
self.onmessage=async({data})=>{
  try {
    await ready;
    let result;
    if(data.op==='png')result=pack_png(JSON.stringify(data.capsule));
    else if(data.op==='svg')result=pack_svg(JSON.stringify(data.capsule));
    else if(data.op==='recover')result=JSON.parse(recover_image(data.bytes));
    else if(data.op==='zip')result=recover_zip(JSON.stringify(data.snapshot));
    else if(data.op==='names')result=JSON.parse(recover_names(data.svg));
    else result=JSON.parse(transmute(JSON.stringify(data.files),data.title,data.minLines,data.palette,data.wingdings));
    self.postMessage({id:data.id,result});
  } catch(error){self.postMessage({id:data.id,error:String(error)});}
};
