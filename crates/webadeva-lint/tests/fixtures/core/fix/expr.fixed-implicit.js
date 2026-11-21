const a = (() => {}) || b;
const c = cond ? () => {} : () => {};
const d = (() => {}) ? 1 : 2;
(() => {
  console.log(1);
})();
!(() => {})();
void (() => {});
const e = (() => {}).call(null);
const f = typeof (() => {});
const g = (() => {})`tpl`;
call(() => {}, (x) => x);
const h = async () => { await h; };
const i = (() => {}).bind(null);
setTimeout(() => {
  run();
}, 1);
const j = a => ({}) && a;
const k = (x) => x ? 1 : 2;
