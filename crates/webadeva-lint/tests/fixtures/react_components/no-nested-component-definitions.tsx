import React, { Component, memo, forwardRef, useCallback } from "react";

function ParentComponent() {
  function UnstableNestedComponent() {
    return <div />;
  }
  return (
    <div>
      <UnstableNestedComponent />
    </div>
  );
}

function ParentComponent2() {
  const UnstableNestedComponent = () => {
    return <div />;
  };
  const Memoized = memo(() => <div />);
  const Forwarded = React.forwardRef((props, ref) => <div ref={ref} />);
  const Callback = useCallback(() => <div />, []);
  return <UnstableNestedComponent />;
}

class ParentClass extends Component {
  render() {
    const Inner = () => <div />;
    function Other() {
      return <span />;
    }
    return <Inner />;
  }
}

class ParentClass2 extends React.Component {
  render() {
    class Unstable extends React.Component {
      render() {
        return <div />;
      }
    }
    return <Unstable />;
  }
}

function ComponentWithProps() {
  return (
    <SomeComponent
      footer={() => <div />}
      header={function Header() {
        return <div />;
      }}
      renderItem={(item) => <div>{item}</div>}
    />
  );
}

function ComponentWithObject() {
  const components = {
    Foo: () => <div />,
    Bar() {
      return <div />;
    },
  };
  return <components.Foo />;
}

function CreateElementProps() {
  return React.createElement(Foo, {
    Footer: () => React.createElement("div"),
  });
}

function MapItems({ items }: { items: string[] }) {
  return (
    <ul>
      {items.map((item) => {
        return <li>{item}</li>;
      })}
    </ul>
  );
}

const TopLevel = () => <div />;
function TopLevel2() {
  return <div />;
}

function useHook() {
  const Comp = () => <div />;
  return Comp;
}

function Outer() {
  const lowercase = () => <div />;
  const _Under = () => <div />;
  const maybe = (x: boolean) => (x ? <div /> : null);
  const Maybe = (x: boolean) => (x ? <div /> : null);
  const Both = (x: boolean) => (x ? <div /> : <span />);
  const Logical = (x: boolean) => x && <div />;
  return null;
}

function WithHooks() {
  function Inner() {
    const [s] = useState(0);
    return s;
  }
  return <Inner />;
}

class Renderer extends Component {
  renderItem = () => {
    const Inner = () => <div />;
    return <Inner />;
  };
  render() {
    return null;
  }
}
