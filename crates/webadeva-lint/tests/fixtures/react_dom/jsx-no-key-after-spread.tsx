const a = <div {...props} key="1" />;
const b = <div key="1" {...props} />;
const c = <div a="b" {...props} b="c" key={x} {...more} key="2" />;
const d = <Foo key="x" />;
const e = <><div {...p} data-key="1" /></>;
