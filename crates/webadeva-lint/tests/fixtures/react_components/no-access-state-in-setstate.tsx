import React from "react";

class A extends React.Component {
  onClick() {
    this.setState({ value: this.state.value + 1 });
  }
  onOther() {
    this.setState(() => ({ value: this.state.value + 1 }));
  }
  ok() {
    this.setState((state) => ({ value: state.value + 1 }));
    const value = this.state.value;
    this.setState({ value });
  }
  nested() {
    this.setState({ a: { b: (this.state as any).c } });
  }
}

class NotComponent {
  setState(x: unknown) {}
  state = { value: 1 };
  run() {
    this.setState({ value: this.state.value });
  }
}

class B extends Component {
  render() {
    this.setState({ x: this.state["x"] });
    this.setState({ y: this?.state.y });
    return null;
  }
}

function fn(this: any) {
  this.setState({ value: this.state.value });
}
