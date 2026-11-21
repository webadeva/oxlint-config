import React, { cloneElement } from "react";
import * as R from "react";
const a = cloneElement(<div />);
const b = React.cloneElement(<div />);
const c = R.cloneElement(<div />);
const d = foo.cloneElement(<div />);
const e = (cloneElement)(<div />);
function F() { return cloneElement(<div />, {}); }
