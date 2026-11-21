import React, { createRef, Component } from "react";

function Comp() {
  const ref = createRef();
  return <div ref={ref} />;
}

const Arrow = () => {
  const ref = React.createRef();
  return <div ref={ref} />;
};

function useThing() {
  const ref = createRef();
  return ref;
}

const useOther = () => createRef();

class Klass extends Component {
  ref = createRef();
  render() {
    return <div ref={this.ref} />;
  }
}

const topLevel = createRef();

function notComponent() {
  const ref = createRef();
  return ref;
}

function Lower() {
  return createRef();
}

function Nested() {
  const handler = () => {
    const r = createRef();
  };
  return <div onClick={handler} />;
}

const Memo = React.memo(() => {
  const r = createRef();
  return <span />;
});

function WithHook() {
  const r = createRef();
  useEffect(() => {});
  return null;
}
