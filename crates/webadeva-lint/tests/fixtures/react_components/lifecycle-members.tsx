import React, { Component, PureComponent } from "react";

class A extends React.Component {
  componentWillMount() {}
  componentWillReceiveProps() {}
  componentWillUpdate() {}
  UNSAFE_componentWillMount() {}
  UNSAFE_componentWillReceiveProps() {}
  UNSAFE_componentWillUpdate() {}
  render() {
    return null;
  }
}

class B extends PureComponent {
  "componentWillMount"() {}
  componentWillReceiveProps = () => {};
  static componentWillUpdate() {}
  ["UNSAFE_componentWillMount"]() {}
  UNSAFE_componentWillReceiveProps = function () {};
}

const C = class extends Component {
  componentWillUpdate() {}
  UNSAFE_componentWillUpdate() {}
};

class NotAComponent {
  componentWillMount() {}
  UNSAFE_componentWillMount() {}
}

class D extends React["Component"] {
  componentWillMount() {}
}
