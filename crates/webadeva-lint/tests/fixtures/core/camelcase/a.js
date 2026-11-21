var first_name = "Nicholas";
var __private_first_name = "Patrick";
var FIRST_NAME = "x", __FOO__ = 1, _foo = 2, foo_ = 3;
function foo_bar(baz_qux, { a_b, c: d_e, f_g = 1 }, [h_i], ...rest_x) { return baz_qux + d_e; }
foo_bar(first_name);
obj.do_something();
do_something();
new do_something_else();
var obj2 = { some_property: 1, "quoted_key": 2, [computed_key]: 3, method_name() {}, get getter_x() { return 1; } };
obj.some_prop = 1;
obj.some_prop2.nested = 1;
x = obj.read_prop;
[obj.arr_target] = [];
({ k: obj.obj_target } = {});
[...obj.rest_target] = [];
[obj.def_target = 1] = [];
obj.upd_target++;
label_name: for (;;) { break label_name; continue label_name; }
try {} catch (err_name) {}
class Class_name { field_name = 1; #priv_name = 2; static static_field; method_x() {} get acc_x() { return this.#priv_name; } }
const Cls = class Expr_class {};
const f = function fn_expr(p_q) {};
const g = (arr_param) => arr_param;
undeclared_global = 1;
var y = undeclared_read;
foo(undeclared_arg);
function d(def_x = default_ref) {}
const { k2 = default_ref2 } = {};
({ shorthand_x } = {});
({ sx: shorthand_y } = {});
({ szz = default_ref3 } = {});
var o3 = { shorthand_z };
for (var for_of_x of []) {}
for (loop_target of []) {}
var a_b_c = 1; a_b_c = 2; a_b_c++; console.log(a_b_c); var zz = a_b_c; zz(a_b_c);
