function plain() { return 1; }
function usesThis() { return this.x; }
function usesArgs() { return arguments.length; }
function* gen() { yield 1; }
async function asyncFn() { await 1; }
const expr = function () { return 2; };
const named = function namedExpr() { return 3; };
const selfRef = function self() { return self; };
const arrowExpr = (a) => a + 1;
const arrowBlock = (a) => { return a + 1; };
const obj = { method() { return 1; }, prop: function () { return 2; }, get g() { return 1; }, set s(v) {}, arrow: () => 1, withThis() { return this; } };
class C { method() { return 1; } static s() {} constructor() {} prop = function () {}; field = () => 1; }
Foo.prototype.bar = function () { return 1; };
Foo.prototype = { baz: function () {} };
function Ctor() {} new Ctor();
const Ctor2 = function () {}; const c2 = new Ctor2();
function hoisted() {} 
early(); function early() {}
function reassigned() {} reassigned = 1;
if (x) function inIf() {}
lbl: function labeled() {}
{ function inBlock() {} }
function outer() { function inner() { return arguments; } return inner; }
function withNested() { const a = () => this; }
function withNestedFn() { return function () { return this; }; }
function dup(a, a) {}
new (function () {})();
class D extends (function () {}) {}
(function iife() {})();
export default function () {}
function newTarget() { return new.target; }
function superUser() { class X extends Y { m() { super.m(); } } }
const o2 = { __proto__() {}, ['computed']: function () {} };
function directive() { "use strict"; return 1; }
const dirArrow = () => { "use strict"; return 1; };
function usedLater() {} const r = usedLater();
callBefore(); function callBefore() {}
const lazy = () => lateFn(); function lateFn() {}
const objX = { f: function () {} }; new objX.f();
function instanceofTarget() {} x instanceof instanceofTarget;
switch (x) { case 1: function inCase() {} }
