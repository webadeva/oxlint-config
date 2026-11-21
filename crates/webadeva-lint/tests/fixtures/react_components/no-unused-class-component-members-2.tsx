import * as React from "react";

class A extends React.Component<{}, {}> {
  handler = () => {
    this.other();
  };
  other() {}
  getterThing() {}
  get computed() {
    return 1;
  }
  set computed(v) {}
  accessor acc = 1;
  declare decl: number;
  render() {
    const { handler } = this;
    const self = this;
    return <div onClick={handler}>{self.getterThing()}</div>;
  }
}

class B extends React.PureComponent {
  static defaultProps = {};
  static getDerivedStateFromProps() {
    return null;
  }
  componentDidCatch() {}
  getSnapshotBeforeUpdate() {
    return null;
  }
  componentWillUnmount() {}
  measure() {}
  setNativeProps() {}
  onlyStaticUse() {}
  static s() {
    this.onlyStaticUse();
  }
}

abstract class C extends React.Component {
  abstract abstractMethod(): void;
  concrete() {}
}

class D extends React.Component {
  constructor(props: any) {
    super(props);
    this.state = {};
    this.counter = 0;
  }
  increment() {
    this.counter++;
  }
}

export default class extends React.Component {
  publicOne() {}
  public explicitPublic() {}
  private priv() {}
}
