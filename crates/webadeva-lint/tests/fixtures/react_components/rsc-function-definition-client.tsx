import x from "y";
"use client";

export function notAsync() {}

function f() {
  "use server";
  return 1;
}

const g = () => {
  "use server";
};
