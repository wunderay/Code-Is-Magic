// Shared outlines may reference only a local, inert path in the root definitions.
export function validateSvg(document) {
  const root=document.documentElement;
  if(document.querySelector('parsererror,script,foreignObject,image')||root.localName!=='svg')throw new Error('Imported SVG contains unsupported elements.');
  const ids=new Map();
  for(const node of document.querySelectorAll('[id]')) {
    if(ids.has(node.id))throw new Error('Imported SVG contains duplicate identifiers.');
    ids.set(node.id,node);
  }
  for(const use of document.querySelectorAll('use')) {
    const href=use.getAttribute('href')||'';
    const target=ids.get(href.slice(1));
    if(!/^#glyph-\d+$/.test(href)||use.children.length||target?.localName!=='path'||target.parentElement?.localName!=='defs'||target.parentElement.parentElement!==root||target.children.length)throw new Error('Imported SVG contains an unsupported outline reference.');
  }
  for(const style of document.querySelectorAll('style')) {
    const allowed=/@font-face\s*\{\s*font-family:\s*'CIM(?:Runic|Symbols|Greek)';\s*src:\s*url\('data:font\/ttf;base64,[A-Za-z0-9+/=]+'\)\s*format\('truetype'\);\s*\}/g;
    if(style.textContent.replace(allowed,'').trim())throw new Error('Imported SVG contains unsupported styling.');
  }
  for(const node of document.querySelectorAll('*'))for(const attr of node.attributes) {
    const localOutline=node.localName==='use'&&attr.name==='href'&&/^#glyph-\d+$/.test(attr.value);
    if(/^on/i.test(attr.name)||(/href/i.test(attr.name)&&!localOutline)||/url\(\s*['"]?(?!#)/i.test(attr.value))throw new Error('Imported SVG contains an external resource or executable attribute.');
  }
}
