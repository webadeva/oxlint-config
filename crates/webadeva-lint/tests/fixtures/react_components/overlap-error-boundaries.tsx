import { use, useState } from "react";
function A({ p }) {
  try {
    const v = use(p);
    return <div>{v}</div>;
  } catch {
    return null;
  }
}
function B() {
  try {
    return <Child />;
  } catch (e) {
    return <Fallback />;
  }
}
function useData(p) {
  try {
    return use(p);
  } catch {
    return null;
  }
}
const C = () => {
  let el;
  try {
    el = <Child />;
  } catch {}
  return el;
};
function D() {
  const handler = () => {
    try {
      return <div />;
    } catch {}
  };
  return null;
}
function notComp() {
  try {
    return <div />;
  } catch {}
}
function E({ items }) {
  try {
    return items.length ? <ul /> : null;
  } finally {
    log();
  }
}
