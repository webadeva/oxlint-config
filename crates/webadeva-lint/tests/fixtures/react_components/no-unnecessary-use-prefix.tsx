import { useState, use } from "react";

function useData() {
  return 1;
}

const useArrow = () => 1;

const useCallsHook = () => {
  const [s] = useState(0);
  return s;
};

function useNested() {
  return () => {
    useState(0);
  };
}

function useEmpty() {}

function use() {
  return 1;
}

function useMDXComponents() {
  return {};
}

function useWithComment() {
  // TODO: use(SomeContext)
  return 1;
}

function useWithComment2() {
  /* call useFoo() later */
  return 1;
}

function useWithBadComment() {
  // use it later
  return 1;
}

jest.mock("./x", () => ({
  useMocked() {
    return 1;
  },
}));

vi.mock("./y", () => {
  const useMock2 = () => 2;
  return { useMock2 };
});

const obj = {
  useMethod() {
    return 1;
  },
};

class K {
  useClassMethod() {
    return 1;
  }
}

function useDirectiveOnly() {
  "use strict";
}

function useTagged() {
  return useTag`x`;
}

function notHook() {
  return 1;
}

function useCallsUse() {
  return use(Promise.resolve(1));
}

export default function useDefault() {
  return 1;
}

function useX1() {
  return 1;
}
