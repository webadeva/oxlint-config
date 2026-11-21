import React, { forwardRef, useContext } from "react";

const Ctx = React.createContext(null);
const A = forwardRef((props, ref) => null);
function B() {
  const v = useContext(Ctx);
  return <Ctx.Provider value={v} />;
}
