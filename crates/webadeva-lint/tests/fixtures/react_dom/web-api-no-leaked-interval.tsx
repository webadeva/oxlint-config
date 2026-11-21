import { useEffect, useLayoutEffect, useInsertionEffect } from "react";

function A() {
  useEffect(() => {
    setInterval(() => {}, 1000);
  }, []);
  return null;
}

function B() {
  useEffect(() => {
    const id = setInterval(() => {}, 1000);
  }, []);
  return null;
}

function C() {
  useEffect(() => {
    const id = setInterval(() => {}, 1000);
    return () => clearInterval(id);
  }, []);
  return null;
}

function D() {
  useLayoutEffect(() => {
    const id = window.setInterval(() => {}, 1000);
    return () => {
      window.clearInterval(id);
    };
  }, []);
  return null;
}

function E() {
  const ref = useRef(0);
  useEffect(() => {
    ref.current = setInterval(() => {}, 1000);
    return () => clearInterval(ref.current);
  }, []);
  useInsertionEffect(() => {
    ref.current = setInterval(() => {}, 1000);
  }, []);
  return null;
}

function F() {
  useEffect(() => {
    let id;
    id = setInterval(() => {}, 1000);
    const id2 = setInterval(() => {}, 1000);
    return () => {
      clearInterval(id);
    };
  }, []);
  return null;
}

function G() {
  setInterval(() => {}, 1000);
  const onClick = () => setInterval(() => {}, 1);
  useEffect(() => {
    const handler = () => {
      const t = setInterval(() => {}, 10);
    };
    if (x) setInterval(() => {}, 1);
  });
  useCustomEffect(() => {
    const id = setInterval(() => {}, 1);
    return () => clearInterval(id);
  });
  React.useEffect(() => {
    const id = setInterval(() => {}, 1);
  });
  useEffect(function () {
    const id = setInterval(() => {}, 1);
    return function () { clearInterval(id) };
  });
  useEffect(() => () => {
    const id = setInterval(() => {}, 1);
  });
  return null;
}
