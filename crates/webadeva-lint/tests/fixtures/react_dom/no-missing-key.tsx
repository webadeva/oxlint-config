import React, { Children } from "react";

const a = [<App />, <App />];
const b = [<App key="1" />, <App key="2" />];
const c = items.map((item) => <App />);
const d = items.map((item) => <App key={item.id} />);
const e = items.map((item) => { return <App />; });
const f = items.map(function (item) { return <App />; });
const g = Array.from(items, (item) => <App />);
const h = items.flatMap((item) => [<App key="a" />]);
const i = items.map((item) => <>{item}</>);
const j = items.map((item) => (cond ? <App /> : <App key="x" />));
const k = items.map((item) => cond && <App />);
const l = items.map((item) => {
  const inner = () => <App />;
  function nested() { return <App />; }
  if (x) return <App />;
  return <App key="1" />;
});
const m = React.Children.toArray([<App />, <App />]);
const n = Children.toArray(items.map((x) => <App />));
const props = { key: "x" };
const o = items.map((item) => <App {...props} />);
const p = items.map((item) => <App {...{ key: item }} />);
const q = items?.map((item) => <App /> as any);
const r = items.forEach((item) => <App />);
const s = items.map((item) => (<App />));
const t = [...items, <App />];
const u = items[map]((item) => <App />);
const v = items.map(renderItem);
const w = [<></>, <React.Fragment key="1" />];
const x2 = items.map((item) => (<App />)!);
