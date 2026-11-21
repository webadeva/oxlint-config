import React, { useState, use } from "react";

function A() {
  const [value, setValue] = useState(0);
  const [isOpen, setIsOpen] = useState(false);
  const [a, setB] = useState(0);
  const [foo, updateFoo] = useState(0);
  const state = useState(0);
  const [, setOnly] = useState(0);
  const [onlyValue] = useState(0);
  const { 0: x } = useState(0);
  const [htmlParser, setHTMLParser] = useState(null);
  const [[nested], setNested] = useState([[1]]);
  const [v2, ...rest] = useState(0);
  useState(0);
  const [c1, setC1] = useState(compute());
  const [c2, setC2] = useState(() => compute());
  const [c3, setC3] = useState(new Map());
  const [c4, setC4] = useState(Boolean(1));
  const [c5, setC5] = useState(obj.compute());
  const [c6, setC6] = useState(useMemoValue());
  const [c7, setC7] = useState([compute(), { a: compute2() }]);
  const [c8, setC8] = useState(use(compute()));
  const [c9, setC9] = useState(a ? compute() : 1);
  const [c10, setC10] = React.useState(compute());
  const [c11, setC11] = useState<number>(0) as any;
  const [c12, setC12] = useState(new Foo.Bar());
  const [c13, setC13] = useState(compute`x`);
  return null;
}

function useCustom() {
  return useState(0);
}

const useArrow = () => useState(0);

function B() {
  const [count, setCount] = useState(0),
    other = useState(1);
  let assigned;
  assigned = useState(0);
  return [count, assigned];
}
