const asT = (() => {}) as unknown as Fn;
const sat = (() => {}) satisfies Fn;
const nn = (() => {})!;
const waiter = async () => {
  await (() => {});
};
const pAlready = (() => {}) || x;
const arrowRet = (a): number /* r */ => { return a };
const arrowNl = (a)
  : number => a;
const cmtArrow = (a) => /* c */ (a);
const o2 = {
  m /* gap */ (a) { return a; },
  [k]: () => {},
  'é': () => { return 'ü'; },
  abc: () => {},
};
const gen = <T>(x: T, ...rest: T[]): T[] => { return [x, ...rest]; };
const defaults = (a = 1, { b } = {}, [c] = []) => { return a + b + c; };
const tagged = (() => {})`x`;
const opt = (() => {})?.();
const cond2 = x ? y : () => {};
const unicodeBody = () => { return "日本語" };
function overload(a: string): string;
function overload(a: any) { return a; }
export const exported = async (): Promise<number> => { return 1; };
let reassigned = () => { return 1; };
const iife = (() => { return 1; })();
const neg = -(() => {});
const del = typeof (() => {}) === 'function';
