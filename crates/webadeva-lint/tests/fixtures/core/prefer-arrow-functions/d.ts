class A extends B {
  early = this.m1();
  static s = this.m2();
  m1() { return 1; }
  m2() { return 2; }
  static m3() { return 3; }
  m4() { return super.m4(); }
  sup() { return 1; }
  'str'() { return 1; }
  [Symbol.iterator]() {}
  #priv() { return 1; }
  ov(a: string): void;
  ov(a: any) {}
  pd(@Inject() x: number) {}
  private acc() {}
}
class G { x() { return super.sup; } }
const keep = { f: function () { return 1; }, g() { doIt(); return 2; } };
const allowedFn = function () {};
obj.allowedProp = function () {};
function allowedDecl() {}
const namedE = function nm() { return 1; };
Proto.prototype.thing = function () {};
const multi = (a) => { const b = a; return b; };
const single = (a) => { return a; };
const bare = () => { return; };
