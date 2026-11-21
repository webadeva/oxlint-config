"use server";

export async function a() {}
export async function b() {}
export async function* gen() {}
export const c = async () => {};
export const d = async () => {};
export const e = async function () {};
export const f = async (x: number) => x, g = async function named() {};
export const h = (async <T,>(x: T) => x) as unknown as () => void;
export const i = (async () => {})!;

async function local1() {}
const local2 = async () => {};
const local3 = async () => {};
export { local1, local2, local3 };

async function withDirective() {
  "use server";
}
export { withDirective };

export default async function () {}
