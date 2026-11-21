const o1 = { get a() {} };
const o2 = { get a() { return; } };
const o3 = { get a() { if (x) return 1; } };
const o4 = { get a() { if (x) { return 1; } else { return 2; } } };
const o5 = { get a() { throw new Error(); } };
const o6 = { get a() { while (true) {} } };
const o7 = { get a() { while (true) { break; } } };
const o8 = { get a() { for (;;) { if (x) return 1; } } };
const o9 = { get a() { switch (x) { case 1: return 1; default: return 2; } } };
const o10 = { get a() { switch (x) { case 1: return 1; case 2: return 2; } } };
const o11 = { get a() { switch (x) { case 1: break; default: return 2; } } };
const o12 = { get a() { try { return 1; } catch (e) { } } };
const o13 = { get a() { try { return y; } catch (e) { } } };
const o14 = { get a() { try { return y; } catch (e) { return 2; } } };
const o15 = { get a() { try { } finally { return 1; } } };
const o16 = { get a() { do { return 1; } while (x); } };
const o17 = { get a() { l: { if (x) break l; return 1; } } };
const o18 = { get a() { for (const k of xs) { return k; } } };
const o19 = { get a() { const f = function () {}; return 1; } };
const o20 = { get a() { const f = () => { return; }; } };
const o21 = { get "str key"() {}, get [computed]() {}, get 1() {}, async *gen() {} };
const o22 = { get a() { return 1; foo(); return; } };
class C {
  get a() {}
  static get b() { return; }
  get #c() { if (x) return 1; }
  get [k]() {}
  set d(v) {}
  get e() { return this.x; }
}
Object.defineProperty(foo, "bar", { get: function () {} });
Object.defineProperty(foo, "bar", { get() { if (x) return 1; } });
Object.defineProperty(foo, "bar", { get: () => {} });
Object.defineProperty(foo, "bar", { get: () => 1 });
Reflect.defineProperty(foo, "bar", { get: function getter() {} });
Object.defineProperties(foo, { bar: { get: function () {} } });
Object.create(foo, { bar: { get() {} } });
Object["defineProperty"](foo, "bar", { get: function () {} });
Object?.defineProperty(foo, "bar", { get: function () {} });
Object.defineProperty(foo, "bar", { "get": function () {} });
Object.defineProperty(foo, { get: function () {} });
foo.defineProperty(foo, "bar", { get: function () {} });
function shadow(Object) { Object.defineProperty(foo, "bar", { get: function () {} }); }
const o23 = { get a() { while (1) { if (x) break; } } };
const o24 = { get a() { do { continue; } while (true); } };
const o25 = { get a() { for (let i = 0; ; i++) { return 1; } } };
const o26 = { get a() { if (x) { return 1; } else throw y; } };
const o27 = { get a() { try { throw 1; } catch { return 2; } } };
const o28 = { get a() { try { x(); } finally { } } };
const o29 = { get a() { outer: for (;;) { for (;;) { break outer; } } } };
const o30 = { get a() { outer: for (;;) { for (;;) { continue outer; } } } };
