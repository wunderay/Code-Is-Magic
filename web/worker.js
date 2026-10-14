import init, { transmute } from './pkg/code_is_magic.js';
let ready = init();
self.onmessage = async ({ data }) => {
  try {
    await ready;
    const result = JSON.parse(transmute(JSON.stringify(data.files), data.title, data.minLines, data.palette, data.wingdings));
    self.postMessage({ id: data.id, result });
  } catch (error) { self.postMessage({ id: data.id, error: String(error) }); }
};
