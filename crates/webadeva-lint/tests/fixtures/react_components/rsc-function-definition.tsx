"use server";

import { db } from "./db";

"use client";

`use server`;

export async function ok() {}

export function notAsync() {}

export const arrow = () => {};

export const asyncArrow = async () => {};

export const fnExpr = function () {};

function local() {}

const localArrow = () => {};

export { local, localArrow };

export default function () {}

function inner() {
  "use server";
}

async function innerOk() {
  "use server";
}

function withClient() {
  "use client";
}

function misplaced() {
  const x = 1;
  "use server";
}

function backtick() {
  `use server`;
}

const objFn = {
  method() {
    "use server";
  },
};

export { other } from "./other";
