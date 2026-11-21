const a = items.map((item) => item.ok ? <A key="1" /> : null);
const b = items.map((item) => [<A />]);
const c = <ul>{items.map((item) => <li>{item}</li>)}</ul>;
const d = items.map((item) => <Fragment>{item}</Fragment>);
const e = items.map(async (item) => <A />);
const f = items.map((item) => item && (item.x || <A />));
const g = Array.from({ length: 3 }, (_, i) => <A key={i} />);
const h = Array.from({ length: 3 }, function () { return <A />; });
const i2 = items.map((item) => {
  switch (item) {
    case 1: return <A />;
    default: return <B key="b" />;
  }
});
