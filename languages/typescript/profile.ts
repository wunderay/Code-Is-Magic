// TypeScript extends the ECMAScript policy and defines its prismatic collar.
interface Polygon {op: 'polygon'; cx: number; cy: number; r: number; n: number; step: number; rotation_degrees: number}
interface Profile {id: string; base: string; name: string; extensions: string[]; call_group: string; color: string; sigil: string; repeats: number; collar: Polygon[]}
const prism = (r: number, rotation_degrees: number): Polygon =>
  ({op: 'polygon', cx: 0, cy: -0.87, r, n: 3, step: 1, rotation_degrees});
const profile: Profile = {
  id: 'typescript', base: 'javascript', name: 'TypeScript',
  extensions: ['ts', 'tsx'], call_group: 'ecmascript', color: '#3178c6', sigil: 'Δ', repeats: 3,
  collar: [prism(0.115, -90), prism(0.07, 90)],
};
console.log(JSON.stringify(profile));
