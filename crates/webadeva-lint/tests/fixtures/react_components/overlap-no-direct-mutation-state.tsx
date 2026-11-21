import React from "react";
class A extends React.Component {
  constructor(p) { super(p); this.state = {}; this.state.a = 1; }
  componentDidMount() { this.state.a = 1; this.state = {}; this.state.b.c = 2; }
  handler = () => { this.state.x = 1; };
  method() { const f = () => { this.state.y = 1; }; this.state.z++; }
  render() { return null; }
}
class B { m() { this.state.a = 1; } }
const C = class extends React.PureComponent { m() { this.state.q = 1; } };
class D extends React.Component { constructor(p) { super(p); const f = () => { this.state.a = 1; }; } }
class E extends React.Component {
  constructor(p) {
    super(p);
    if (p.x) { this.state.a = 1; }
    this.state.b = 2;
    this["state"].c = 3;
    (this.state as any).d = 4;
  }
  m() { this.state["e"] = 5; [this.state.f] = [1]; }
}
