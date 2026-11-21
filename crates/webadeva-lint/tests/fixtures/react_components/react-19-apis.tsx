import React, { forwardRef, useContext, createContext } from "react";
import * as R from "react";

const ThemeContext = createContext(null);
const Ctx = { Context: ThemeContext };

export function App({ children }: { children: React.ReactNode }) {
  const theme = useContext(ThemeContext);
  const other = React.useContext(ThemeContext);
  const third = R.useContext(ThemeContext);
  const four = (useContext)(ThemeContext);
  return (
    <>
      <ThemeContext.Provider value={theme}>{children}</ThemeContext.Provider>
      <Ctx.Context.Provider value={other} />
      <Foo.Provider value={third} />
      <Provider value={four} />
      <ThemeContext value={1} />
      <a.b.SomeContext.Provider />
    </>
  );
}

const Input = forwardRef((props, ref) => <input ref={ref} {...props} />);
const Input2 = React.forwardRef<HTMLInputElement, {}>(function Input2(props, ref) {
  return <input ref={ref} {...props} />;
});
const Wrapped = React.memo(forwardRef((props, ref) => null));
let assigned;
assigned = forwardRef(() => null);
const obj = { key: forwardRef(() => null) };
export default forwardRef(() => null);
const typed = forwardRef(() => null) as any;
const cond = x ? forwardRef(() => null) : null;
