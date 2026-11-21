const a = <div dangerouslySetInnerHTML={{ __html: "x" }}>Children</div>;
const b = <div dangerouslySetInnerHTML={{ __html: "x" }} children="x" />;
const c = (
  <div dangerouslySetInnerHTML={{ __html: "x" }}>
  </div>
);
const d = <div dangerouslySetInnerHTML={{ __html: "x" }}> </div>;
const e = <div dangerouslySetInnerHTML={{ __html: "x" }} />;
const props = { children: "x", dangerouslySetInnerHTML: { __html: "x" } };
const f = <div {...props} />;
const g = (
  <Foo dangerouslySetInnerHTML={{ __html: "x" }}>
    <span />
  </Foo>
);
const h = <div>{x}</div>;
