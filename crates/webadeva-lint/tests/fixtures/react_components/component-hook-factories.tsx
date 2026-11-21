import { Component, PureComponent, useState, memo, forwardRef } from "react";

export function Top() {
  return <div />;
}

export function useTop() {
  return useState(0);
}

export function makeComponent(label: string) {
  function Inner() {
    return <span>{label}</span>;
  }
  const Arrow = () => <b>{label}</b>;
  const Memo = memo(() => <i>{label}</i>);
  const Ref = forwardRef<HTMLDivElement>((_, ref) => <div ref={ref} />);
  return [Inner, Arrow, Memo, Ref];
}

export function makeHook(initial: number) {
  function useCounter() {
    const [n, setN] = useState(initial);
    return { n, setN };
  }
  const useOther = () => useState(initial);
  const obj = { useProp: () => useState(1) };
  return [useCounter, useOther, obj];
}

export function makeClass() {
  class Inner extends Component {
    render() {
      return <div />;
    }
  }
  const Expr = class extends PureComponent {
    render() {
      return null;
    }
  };
  return [Inner, Expr, class extends Component {}];
}

export const factory = () => {
  const Maybe = () => (Math.random() > 0.5 ? <div /> : null);
  const Num = () => 1;
  const Str = () => "x";
  const Both = (a: boolean) => a && <div />;
  return [Maybe, Num, Str, Both];
};

export function lists(items: string[]) {
  const a = items.map((i) => <li key={i}>{i}</li>);
  const b = items.flatMap((i) => [<li key={i}>{i}</li>]);
  const [First] = [() => <div />];
  const arr = [() => <div />];
  return [a, b, First, arr];
}

export class Holder extends Component {
  Field = () => <div />;
  method() {
    const Local = () => <div />;
    return Local;
  }
  render() {
    return <div />;
  }
}

export function outer() {
  return function Named() {
    return <div />;
  };
}

export default function () {
  function useLocal() {
    return useState(0);
  }
  return useLocal;
}
