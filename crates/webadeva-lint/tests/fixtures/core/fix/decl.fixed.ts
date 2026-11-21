const add = (a: number, b: number): number => {
  return a + b;
};
export const named = <T>(x: T): T => {
  return x;
};
export default (a, b) => {
  return a + b;
};
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
): string => {
  return a + b;
};
function breaksArrow()
  : number {
  return 1;
}
const nested = (x) => {
  return (y) => {
    return x + y;
  };
};
const arrow = (x) => { return x * 2 };
const bare = (x) => { return x };
const asyncBare = async (y) => { return y };
const typed = <T>(v: T): T => { return v };
const obj = () => { return { a: 1 } };
const seq = () => { return 1, 2 };
const commented = () => /* c */ 1;
const parenComment = () => (/* c */ 1);
const deep = (a) => { return (b) => { return (c) => { return a + b + c } } };
const o = {
  method: (a) => {
    return a;
  },
  prop: (b) => {
    return b;
  },
  am: async () => {
    await 1;
  },
  'str-key': () => {},
  [computed]: () => {},
  42: () => {},
  key: /* k */ function () {},
};
class K {
  m() {
    return 1;
  }
}
