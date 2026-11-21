import React, { Component, PureComponent } from "react";

class A extends Component {
  UNSAFE_componentWillMount() {}
  UNSAFE_componentWillReceiveProps(next: unknown) {}
  UNSAFE_componentWillUpdate() {}
  render() {
    return null;
  }
}

class B extends React.PureComponent {
  UNSAFE_componentWillMount() {}
  UNSAFE_componentWillReceiveProps = () => {};
  UNSAFE_componentWillUpdate = function () {};
  static componentWillMount() {}
  ["componentWillUpdate"]() {}
  render() {
    return null;
  }
}

const C = class extends PureComponent {
  public UNSAFE_componentWillMount(): void {}
  protected async UNSAFE_componentWillUpdate() {}
  /* keep */ UNSAFE_componentWillReceiveProps /* me */ () {}
  render() {
    return null;
  }
};

class NotAComponent {
  componentWillMount() {}
}

class D extends Component {
  UNSAFE_componentWillMount() {}
  UNSAFE_componentWillMount() {}
  render() {
    return null;
  }
}

export { A, B, C, D, NotAComponent };
