function add(a: number, b: number): number {
  return a + b;
}
export function named<T>(x: T): T {
  return x;
}
export default function (a, b) {
  return a + b;
}
async function load(url: string): Promise<void> {
  await fetch(url);
}
function withComment(a /* first */, b) {
  // body comment
  return a;
}
function /* lost */ dropped() {
  return 1;
}
function multiline(
  a: string,
  b: number,
): string {
  return a + b;
}
function breaksArrow()
  : number {
  return 1;
}
const nested = function (x) {
  return function (y) {
    return x + y;
  };
};
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
  method(a) {
    return a;
  },
  prop: function (b) {
    return b;
  },
  async am() {
    await 1;
  },
  'str-key': function () {},
  [computed]: function () {},
  42() {},
  key: /* k */ function () {},
};
class K {
  m() {
    return 1;
  }
}
