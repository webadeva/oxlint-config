"use server";

export function a() {}
export async function b() {}
export function* gen() {}
export const c = () => {};
export const d = async () => {};
export const e = function () {};
export const f = (x: number) => x, g = function named() {};
export const h = (<T,>(x: T) => x) as unknown as () => void;
export const i = (() => {})!;

function local1() {}
const local2 = () => {};
const local3 = async () => {};
export { local1, local2, local3 };

function withDirective() {
  "use server";
}
export { withDirective };

export default function () {}
