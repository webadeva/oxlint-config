import React, { forwardRef, useState, useRef, createContext, useId } from "react";

const A = (forwardRef((props, ref) => null));
const B = ((props: any) => <div />) as any;
const ctx = (createContext(null));
const Ctx2 = ((createContext(null)!));

function Parent() {
  const Nested = (() => <div />);
  const [v, setV] = (useState(0));
  const [w, setW] = useState((compute()));
  const r = (useRef(null));
  const i = (useId()) as string;
  const s = (useState(0));
  return null;
}

class K extends React.Component {
  componentDidMount() {
    (this.setState)({ a: (this.state).a });
    this.setState({ b: ((this).state).b });
  }
  render() {
    const Inner = (() => <div />);
    return null;
  }
}

function useNothing() {
  return (1);
}
