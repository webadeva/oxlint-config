"use client";
import { useState, useEffect } from "react";

function Clock() {
  const now = Date.now();
  const r = Math.random();
  const d = new Date();
  const fixed = new Date(2020, 1, 1);
  const id = crypto.randomUUID();
  const w = window.innerWidth;
  const t = window.setTimeout(() => {}, 1);
  const p = performance.now();
  const g = globalThis.Math.random();
  const nested = window.Math.random();
  const ok = Math.floor(1.5);
  const ok2 = JSON.stringify({});
  fetch("/x");
  setTimeout(() => {});
  useEffect(() => {
    Math.random();
    new Date();
  }, []);
  const handler = () => Date.now();
  return <div onClick={() => Math.random()}>{now}</div>;
}

function useNow() {
  return new Date();
}

function useRandom() {
  const { random } = Math;
  const { now: getNow = () => 0 } = Date;
  const M = Math;
  const win = window;
  const loc = win.localStorage;
  random();
  getNow();
  M.random();
  win.fetch("/a");
  loc.getItem("x");
  const [first] = [Math];
  first.random();
  return useState(0);
}

function helper() {
  return Math.random();
}

async function AsyncComp() {
  const x = Date.now();
  return <div>{x}</div>;
}

const Arrow = () => {
  const o = new IntersectionObserver(() => {});
  const ws = new WebSocket("ws://x");
  const m = new Map();
  Object.assign({}, {});
  Object.keys({});
  document.getElementById("a");
  localStorage.getItem("k");
  return <span />;
};

function Shadowed() {
  const Math = { random: () => 4 };
  const r = Math.random();
  return <div>{r}</div>;
}

class K {
  render() {
    return Math.random();
  }
}
