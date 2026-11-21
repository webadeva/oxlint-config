import { useState } from "react";
function A() {
  const [s, setS] = useState(0);
  setS(1);
  if (s) setS(2);
  const handler = () => setS(3);
  return <div onClick={handler}>{s}</div>;
}
function useH() {
  const [s, setS] = useState(0);
  setS(1);
  return s;
}
function B() {
  const state = useState(0);
  state[1](1);
  state.at(1)(2);
  return null;
}
function C({ p }) {
  const [s, setS] = useState(0);
  if (p) return null;
  setS(1);
  return null;
}
function D() {
  const [s, setS] = useState(0);
  s !== 1 && setS(1);
  return null;
}
const E = () => {
  const [, setX] = useState(0);
  setX((x) => x + 1);
  return null;
};
function F() {
  const [s, setS] = useState(0);
  for (const i of [1]) setS(i);
  try { setS(2); } catch {}
  return null;
}
function G({ p }) {
  const [s, setS] = useState(0);
  const { 1: setO } = useState(0) as any;
  setO(1);
  switch (p) { case 1: setS(1); }
  while (false) setS(2);
  return null;
}
const H = memo(() => {
  const [s, setS] = useState(0);
  setS(1);
  return null;
});
const obj = { Comp() { const [s, setS] = useState(0); setS(1); return null; } };
