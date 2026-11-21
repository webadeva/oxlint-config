import React, { Component, PureComponent } from "react";

class Foo extends React.Component {
  handleClick() {}
  used() {}
  prop = 1;
  arrow = () => {};
  static staticMethod() {}
  state = {};
  constructor(props: any) {
    super(props);
    this.assigned = 1;
    this.usedAssigned = 2;
  }
  componentDidMount() {
    this.used();
    console.log(this.usedAssigned);
    const { destructured } = this;
  }
  destructured() {}
  render() {
    return <div />;
  }
  shouldComponentUpdate() {
    return true;
  }
  focus() {}
  #privateThing() {}
  ["computed"]() {}
}

class Pure extends PureComponent {
  shouldComponentUpdate() {
    return true;
  }
  render() {
    return null;
  }
}

export class Exported extends Component {
  publicMethod() {}
  private privateMethod() {}
  protected protectedMethod() {}
  publicArrow = () => {};
  publicValue = 1;
  render() {
    return null;
  }
}

export default class DefaultExported extends Component {
  method() {}
  render() {
    return null;
  }
}

export const VarExported = class extends Component {
  method() {}
};

const Anonymous = class extends Component {
  unusedInAnon() {}
};

(class extends Component {
  unusedNoName() {}
});

class NotComponent {
  unused() {}
}

class Nested extends Component {
  outer() {
    class Inner extends Component {
      innerUnused() {}
      render() {
        this.outer2();
        return null;
      }
    }
    return Inner;
  }
  outer2() {}
}
