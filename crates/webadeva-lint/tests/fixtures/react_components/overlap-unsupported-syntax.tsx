function A() {
  eval("1");
  globalThis.eval("2");
  return null;
}
function useH() {
  eval("x");
}
function notComp() {
  eval("y");
}
const B = () => { (0, eval)("z"); window.eval("w"); return <div />; };
function C() { const f = () => eval("n"); return null; }
