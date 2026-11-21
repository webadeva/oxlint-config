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
call(() => {}, (x) => { return x; });
const h = async () => { await h; };
const i = (() => {}).bind(null);
setTimeout(() => {
  run();
}, 1);
const j = (a) => { return ({}) && a };
const k = (x) => { return x ? 1 : 2 };
