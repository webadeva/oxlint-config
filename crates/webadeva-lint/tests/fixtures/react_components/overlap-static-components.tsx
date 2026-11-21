import { memo, useMemo } from "react";
function A() {
  const Inner = () => <div />;
  return <Inner />;
}
function B() {
  function Inner() { return <div />; }
  return <Inner />;
}
function C({ type }) {
  const Comp = components[type];
  return <Comp />;
}
function D() {
  const Comp = memo(() => <div />);
  return <Comp />;
}
function E() {
  const Comp = useMemo(() => () => <div />, []);
  return <Comp />;
}
function F() {
  let Comp = Base;
  if (x) Comp = () => <div />;
  return <Comp />;
}
function G() {
  const Comp = makeComponent();
  return <Comp />;
}
class K extends React.Component {
  render() {
    const Inner = () => <div />;
    return <Inner />;
  }
}
function useH() {
  const Inner = () => <div />;
  return <Inner />;
}
function H() {
  const Comp = cond ? () => <a /> : Base;
  return <Comp />;
}
