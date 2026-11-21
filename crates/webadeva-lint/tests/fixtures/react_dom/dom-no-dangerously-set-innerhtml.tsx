const a = <div dangerouslySetInnerHTML={{ __html: "x" }} />;
const b = <App dangerouslySetInnerHTML={{ __html: "x" }} />;
const props = { dangerouslySetInnerHTML: { __html: "x" } };
const c = <div {...props} />;
const d = <div dangerouslySetInnerHTML />;
const e = <div />;
