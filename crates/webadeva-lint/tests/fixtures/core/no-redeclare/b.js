/* global foo, Array, bar:writable */
/* global foo */
var foo = 1;
var bar = 2;
var baz = 1;
var baz = 2;
function x(a) { var a; }
let window = 1;
export { foo, bar, baz, x, window };
