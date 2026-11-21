const asT = (() => {}) as unknown as Fn;
const sat = (() => {}) satisfies Fn;
const nn = (() => {})!;
const waiter = async () => {
  await (() => {});
};
const pAlready = (() => {}) || x;
const arrowRet = (a): number /* r */ => a;
const arrowNl = (a)
  : number => a;
const cmtArrow = (a) => /* c */ (a);
const o2 = {
  m /* gap */ (a) { return a; },
  [k]: () => {},
  'é': () => 'ü',
  abc: () => {},
};
const gen = <T>(x: T, ...rest: T[]): T[] => [x, ...rest];
const defaults = (a = 1, { b } = {}, [c] = []) => a + b + c;
const tagged = (() => {})`x`;
const opt = (() => {})?.();
const cond2 = x ? y : () => {};
const unicodeBody = () => "日本語";
function overload(a: string): string;
function overload(a: any) { return a; }
export const exported = async (): Promise<number> => 1;
let reassigned = () => 1;
const iife = (() => 1)();
const neg = -(() => {});
const del = typeof (() => {}) === 'function';
