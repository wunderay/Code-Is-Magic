// ECMAScript shares brace heuristics; declarations, dialect pairing and solar design live here.
const profile = {
  id: 'javascript', base: 'c', name: 'JavaScript',
  extensions: ['js', 'jsx', 'mjs', 'cjs'], call_group: 'ecmascript',
  object_keywords: ['const', 'let', 'var'], arrow_token: '=>',
  sigil: '☉', repeats: 12,
  collar: [{op: 'path', data: 'M 0 -.99 L 0 -.94 M -.018 -.982 L .018 -.947 M .018 -.982 L -.018 -.947'}],
};
console.log(JSON.stringify(profile));
