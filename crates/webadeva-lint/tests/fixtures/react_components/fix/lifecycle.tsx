import React, { Component, PureComponent } from "react";

class A extends Component {
  componentWillMount() {}
  componentWillReceiveProps(next: unknown) {}
  componentWillUpdate() {}
  render() {
    return null;
  }
}

class B extends React.PureComponent {
  "componentWillMount"() {}
  'componentWillReceiveProps' = () => {};
  componentWillUpdate = function () {};
  static componentWillMount() {}
  ["componentWillUpdate"]() {}
  render() {
    return null;
  }
}

const C = class extends PureComponent {
  public componentWillMount(): void {}
  protected async componentWillUpdate() {}
  /* keep */ componentWillReceiveProps /* me */ () {}
  render() {
    return null;
  }
};

class NotAComponent {
  componentWillMount() {}
}

class D extends Component {
  UNSAFE_componentWillMount() {}
  componentWillMount() {}
  render() {
    return null;
  }
}

export { A, B, C, D, NotAComponent };
