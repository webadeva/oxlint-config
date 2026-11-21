import React, { createContext, useId, useRef } from "react";

const ThemeContext = createContext(null);
const theme = createContext(null);
const ThemeCtx = React.createContext(null);
const contexts = { Theme: createContext(null), BadContext: createContext(null), "StrContext": createContext(null) };
let assigned;
assigned = createContext(null);
ns.SomeContext = createContext(null);
ns.some = createContext(null);
const maybe = cond ? createContext(null) : null;
const typed = createContext(null) as any;
const seq = (0, createContext(null));
const wrapped = wrap(createContext(null));
class K {
  ctx = createContext(null);
  static FooContext = createContext(null);
}

function Comp() {
  const id = useId();
  const inputId = useId();
  const bad = useId();
  const ref = useRef(null);
  const inputRef = useRef(null);
  const badRef = React.useRef(null);
  const nope = useRef(null);
  const value = useRef(null).current;
  const refs = { a: useRef(null), aRef: useRef(null) };
  const [destructured] = useRef(null) as any;
  const both = useId() || useRef(null);
  const awaited = useMemo(() => useRef(0), []);
  return null;
}
