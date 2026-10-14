import init, { transmute_keyed, pack_png_keyed, pack_svg_keyed, pack_svgz_keyed, recover_image_keyed, recover_zip, recover_names_keyed, optimize_png } from './pkg/code_is_magic.js';
const ready=init();
self.onmessage=async({data})=>{
  try {
    await ready;
    let result;
    if(data.op==='png')result=pack_png_keyed(JSON.stringify(data.capsule),data.key||'');
    else if(data.op==='optimize-png')result=optimize_png(data.bytes);
    else if(data.op==='svg')result=pack_svg_keyed(JSON.stringify(data.capsule),data.key||'');
    else if(data.op==='svgz')result=pack_svgz_keyed(JSON.stringify(data.capsule),data.key||'');
    else if(data.op==='recover')result=JSON.parse(recover_image_keyed(data.bytes,data.key||''));
    else if(data.op==='zip')result=recover_zip(JSON.stringify(data.snapshot));
    else if(data.op==='names')result=JSON.parse(recover_names_keyed(data.svg,data.key||''));
    else result=JSON.parse(transmute_keyed(JSON.stringify(data.files),data.title,data.minLines,data.palette,data.wingdings,!!data.languageColors,data.key||''));
    self.postMessage({id:data.id,result});
  } catch(error){self.postMessage({id:data.id,error:String(error)});}
};
