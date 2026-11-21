type Type_alias = { prop_x: string };
interface Iface_name { method_sig(arg_x: number): void; prop_y: number }
enum Enum_name { Member_x }
namespace Ns_name { export const inner_x = 1; }
declare function declared_fn(declared_param: string): void;
function overload_fn(a: string): void;
function overload_fn(a: number): void;
function overload_fn(over_param: any) {}
let typed_var: Type_alias;
let typed_var2: Iface_name = {} as Iface_name;
let ref_t: Unresolved_type;
function gen<Type_param>(typed_param: Type_param, opt_param?: string, ...rest_p: number[]) {}
class Cls_x { constructor(private param_prop: string, public readonly ro_prop = 1) {} abstract_like?: string; declare declared_field: number; }
abstract class Abs_x { abstract abstract_method(): void; abstract abstract_prop: string; accessor accessor_prop = 1; }
type Fn_type = (fn_type_param: string) => void;
const arrow_typed = (p_typed: string): void => {};
try {} catch (catch_typed: unknown) {}
import type { Type_import } from "x";
import Eq_name = require("x");
let tq: typeof typed_var;
export = typed_var;
