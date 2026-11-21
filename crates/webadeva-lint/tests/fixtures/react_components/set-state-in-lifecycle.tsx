import React from "react";

class A extends React.Component {
  componentDidMount() {
    this.setState({ a: 1 });
    setTimeout(() => this.setState({ a: 2 }));
    const f = function (this: any) {
      this.setState({ a: 3 });
    };
    if (true) {
      this.setState({ a: 4 });
    }
  }
  componentDidUpdate() {
    this.setState({ b: 1 });
    this.props.fetch().then(() => this.setState({ b: 2 }));
  }
  componentWillUpdate() {
    this.setState({ c: 1 });
  }
  UNSAFE_componentWillUpdate() {
    this.setState({ c: 2 });
  }
  static componentDidMount() {
    this.setState({ d: 1 });
  }
  other() {
    this.setState({ e: 1 });
  }
}

class B extends React.PureComponent {
  componentDidMount = () => {
    this.setState({ a: 1 });
  };
  componentDidUpdate = function (this: any) {
    this.setState({ b: 1 });
  };
}

class NotComponent {
  componentDidMount() {
    this.setState({ a: 1 });
  }
  setState(x: unknown) {}
}

class C extends React.Component {
  componentDidMount() {
    class Inner {
      componentDidMount() {
        (this as any).setState({});
      }
    }
    this?.setState({ z: 1 });
  }
}
