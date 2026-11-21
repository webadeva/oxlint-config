const asT = function () {} as unknown as Fn;
const sat = function () {} satisfies Fn;
const nn = function () {}!;
async function waiter() {
  await function () {};
}
const pAlready = (function () {}) || x;
const arrowRet = (a): number /* r */ => a;
const arrowNl = (a)
  : number => a;
const cmtArrow = (a) => /* c */ (a);
const o2 = {
  m /* gap */ (a) { return a; },
  [(k)]: function () {},
  'é'() { return 'ü'; },
  abc() {},
};
function gen<T>(x: T, ...rest: T[]): T[] { return [x, ...rest]; }
function defaults(a = 1, { b } = {}, [c] = []) { return a + b + c; }
const tagged = function () {}`x`;
const opt = function () {}?.();
const cond2 = x ? y : function () {};
const unicodeBody = () => "日本語";
function overload(a: string): string;
function overload(a: any) { return a; }
export async function exported(): Promise<number> { return 1; }
let reassigned = function () { return 1; };
const iife = function () { return 1; }();
const neg = -function () {};
const del = typeof function () {} === 'function';
