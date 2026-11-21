function overloaded(a: string): void;
function overloaded(a: number): void;
function overloaded(a: any) {}
export function expOver(a: string): void;
export function expOver(a: any) {}
function noSemi(a: string): void
function noSemi(a: any) {}
function assertsFn(x: unknown): asserts x is string {}
function predicate(x: unknown): x is string { return true; }
function withThisParam(this: Window) {}
function generic<T>(x: T): T { return x; }
const typed = (x: number): number => x * 2;
export function exported() {}
export default function namedDefault() {}
class E { m(): void {} overloadedM(a: string): void; overloadedM(a: any) {} @dec decorated() {} }
abstract class F { abstract m(): void; }
namespace N { function inNs() {} }
declare function declared(): void;
const fnInGeneric = <T,>(x: T) => x;
