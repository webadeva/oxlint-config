const a = function () {} || b;
const c = cond ? function () {} : function () {};
const d = function () {} ? 1 : 2;
(function () {
  console.log(1);
})();
!function () {}();
void function () {};
const e = function () {}.call(null);
const f = typeof function () {};
const g = function () {}`tpl`;
call(function () {}, function (x) { return x; });
const h = async function () { await h; };
const i = (function () {}).bind(null);
setTimeout(function () {
  run();
}, 1);
const j = a => ({}) && a;
const k = (x) => x ? 1 : 2;
