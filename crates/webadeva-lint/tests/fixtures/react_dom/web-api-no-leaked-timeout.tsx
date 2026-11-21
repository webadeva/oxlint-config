import { useEffect, useLayoutEffect, useInsertionEffect } from "react";

function A() {
  useEffect(() => {
    setTimeout(() => {}, 1000);
  }, []);
  return null;
}

function B() {
  useEffect(() => {
    const id = setTimeout(() => {}, 1000);
  }, []);
  return null;
}

function C() {
  useEffect(() => {
    const id = setTimeout(() => {}, 1000);
    return () => clearTimeout(id);
  }, []);
  return null;
}

function D() {
  useLayoutEffect(() => {
    const id = window.setTimeout(() => {}, 1000);
    return () => {
      window.clearTimeout(id);
    };
  }, []);
  return null;
}

function E() {
  const ref = useRef(0);
  useEffect(() => {
    ref.current = setTimeout(() => {}, 1000);
    return () => clearTimeout(ref.current);
  }, []);
  useInsertionEffect(() => {
    ref.current = setTimeout(() => {}, 1000);
  }, []);
  return null;
}

function F() {
  useEffect(() => {
    let id;
    id = setTimeout(() => {}, 1000);
    const id2 = setTimeout(() => {}, 1000);
    return () => {
      clearTimeout(id);
    };
  }, []);
  return null;
}

function G() {
  setTimeout(() => {}, 1000);
  const onClick = () => setTimeout(() => {}, 1);
  useEffect(() => {
    const handler = () => {
      const t = setTimeout(() => {}, 10);
    };
    if (x) setTimeout(() => {}, 1);
  });
  useCustomEffect(() => {
    const id = setTimeout(() => {}, 1);
    return () => clearTimeout(id);
  });
  React.useEffect(() => {
    const id = setTimeout(() => {}, 1);
  });
  useEffect(function () {
    const id = setTimeout(() => {}, 1);
    return function () { clearTimeout(id) };
  });
  useEffect(() => () => {
    const id = setTimeout(() => {}, 1);
  });
  return null;
}
