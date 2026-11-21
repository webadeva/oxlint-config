import { lazy, Component } from "react";
import React from "react";

const Top = lazy(() => import("./Top"));

function App() {
  const Nested = lazy(() => import("./Nested"));
  const Also = React.lazy(async () => {
    const mod = await import("./Also");
    const other = await import("./Other");
    return mod;
  });
  return <Nested />;
}

function useLazy() {
  const L = lazy(() => import("./L"));
  return L;
}

class Klass extends Component {
  L = lazy(() => import("./K"));
  render() {
    return null;
  }
}

function notComponent() {
  return lazy(() => import("./NC"));
}

const Arrow = () => {
  const L = lazy(() => import("./Arrow"));
  return <L />;
};

const noImport = () => {
  const L = lazy(() => Promise.resolve({ default: () => null }));
  return <L />;
};
