import { useMemo } from "react";
function A({ a }) {
  useMemo(() => a, [a]);
  const b = useMemo((x) => x, []);
  const c = useMemo(async () => 1, []);
  const d = useMemo(function* () { yield 1; }, []);
  let outer;
  const e = useMemo(() => { outer = 1; return 2; }, []);
  const f = useMemo(() => { a.b = 1; }, []);
  const g = useMemo(() => { if (a) return 1; }, []);
  const h = useMemo(() => { const i = 1; return; }, []);
  return null;
}
function notComp() {
  useMemo(() => 1, []);
  const x = useMemo(() => {}, []);
}
const y = useMemo((p) => p, []);
function B() {
  const v = React.useMemo(() => {}, []);
  return v;
}
async function C() {
  let n = 0;
  const z = useMemo(() => { for (n of [1]) {} [n] = [2]; ({ n } = { n: 3 }); const f = () => { n = 4; }; return n; }, []);
  const w = useMemo(function () { function inner() { return 1; } }, []);
  const v = await useMemo(() => 1, []);
  return <div>{useMemo(() => 1, [])}</div>;
}
