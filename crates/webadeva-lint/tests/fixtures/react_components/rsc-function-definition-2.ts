"use server";

export default async function () {}

export class NotAFunction {}

export const obj = { a: () => {} };

const fnExpr = function named() {};
export { fnExpr as renamed };

export const typed = (() => {}) as () => void;

export let maybe = cond ? () => {} : async () => {};

function hoisted() {}
export { hoisted as default };

export async function* gen() {}

export function* syncGen() {}
