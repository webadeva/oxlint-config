const add = (a: number, b: number): number => a + b;
export const named = <T>(x: T): T => x;
export default (a, b) => a + b;
const load = async (url: string): Promise<void> => {
  await fetch(url);
};
const withComment = (a /* first */, b) => {
  // body comment
  return a;
};
function /* lost */ dropped() {
  return 1;
}
const multiline = (
  a: string,
  b: number,
): string => a + b;
function breaksArrow()
  : number {
  return 1;
}
const nested = (x) => (y) => x + y;
const arrow = (x) => x * 2;
const bare = x => x;
const asyncBare = async y => y;
const typed = <T>(v: T): T => v;
const obj = () => ({ a: 1 });
const seq = () => (1, 2);
const commented = () => /* c */ 1;
const parenComment = () => (/* c */ 1);
const deep = a => b => c => a + b + c;
const o = {
  method: (a) => a,
  prop: (b) => b,
  am: async () => {
    await 1;
  },
  'str-key': () => {},
  [computed]: () => {},
  42: () => {},
  key: /* k */ function () {},
};
class K {
  m = () => 1;
}
