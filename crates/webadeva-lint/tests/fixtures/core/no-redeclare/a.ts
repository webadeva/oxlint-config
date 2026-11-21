export const A = { X: "x" } as const;
export type A = (typeof A)[keyof typeof A];
export type B = string;
export const B = 1;
interface C { a: 1 }
interface C { b: 1 }
class D {}
interface D { c: 1 }
namespace E { export const q = 1; var q2 = 1; var q2 = 2; }
function E() {}
function F(a: string): void;
function F(a: number): void;
function F(a: unknown) { return a; }
const G = 1;
type G = number;
interface G { z: 1 }
function H(p: number) { var h = 1; var h = 2; var p = 3; return h + p; }
enum I { a }
enum I { b = 2 }
namespace I { export const c = 3; }
declare module "m" { const x: number; const x: string; }
declare global { var gx: number; interface Window { w1: 1 } }
interface Window { w2: 2 }
var Promise = 1;
const self = 2;
import { J } from "./j";
type J = number;
{ var k = 1; var k = 2; type T = 1; interface T {} }
switch (A) { case "x": var s = 1; var s = 2; }
for (var f = 0; ; ) { var f = 1; break; }
for (const z of []) { let y = z; { let y = 2; } }
try {} catch (e) { var e2 = 1; var e2 = 2; }
class K { static { var st = 1; var st = 2; } m() { var mm; var mm; } }
const L = function L2() { var L2 = 1; return L2; };
const M = () => { var m1; var m1; };
abstract class N {}
interface N { n: 1 }
function O(this: void, o: number, o2: number) {}
declare function P(): void;
declare function P(x: number): void;
type Q<T> = T; type R<Q> = Q;
export { C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, gx };
