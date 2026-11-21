import React from "react";
import { connect } from "react-redux";

export default function Page() {
  const Connected = connect(mapState)(() => <div />);
  const Observed = observer(function () {
    return <div />;
  });
  const Unknown = someHoc(() => <div />);
  class Inner extends React.Component {
    render() {
      return <div />;
    }
  }
  const items = [1, 2].map((i) => <li key={i}>{i}</li>);
  const Fragmented = () => <></>;
  const Nullish = () => null;
  const Str = () => "text";
  const CreateEl = () => React.createElement("div");
  const element = React.createElement(Wrapper, null, () => <div />);
  return <Connected />;
}

function NotAComponent() {
  return 42;
}

function Outer({ children }) {
  return (
    <List
      renderRow={function Row() {
        return <tr />;
      }}
      Item={() => <li />}
      items={{ Cell: () => <td /> }}
    >
      {() => <div />}
    </List>
  );
}

const obj = {
  Panel: () => <div />,
};

function makeThing() {
  const Local = () => <div />;
  return Local;
}

function WithMemo() {
  const M = React.memo(function M() {
    return <div />;
  });
  const N = React.memo(React.forwardRef(() => <div />));
  return null;
}

const Hoc = (Wrapped) => {
  const Enhanced = (props) => <Wrapped {...props} />;
  return Enhanced;
};

function Async() {
  const Later = async () => {
    return <div />;
  };
  return null;
}

function Seq() {
  const Seqd = () => (0, <div />);
  const Logic = () => a && <div />;
  const Arr = () => [<div />, <span />];
  const ArrMixed = () => [<div />, 1];
  const Ident = () => {
    const el = <div />;
    return el;
  };
  return null;
}
