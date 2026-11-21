import { useEffect, useLayoutEffect } from "react";

function A() {
  useEffect(() => {
    window.addEventListener("resize", () => {});
  }, []);
  return null;
}

function B() {
  useEffect(() => {
    const handlerB = () => {};
    window.addEventListener("resize", handlerB);
    return () => window.removeEventListener("resize", handlerB);
  }, []);
  return null;
}

function C() {
  useEffect(() => {
    const handlerC = () => {};
    window.addEventListener("resize", handlerC);
  }, []);
  return null;
}

function D() {
  useEffect(() => {
    const handlerD = () => {};
    window.addEventListener("resize", handlerD, true);
    return () => window.removeEventListener("resize", handlerD);
  }, []);
  return null;
}

function E() {
  useEffect(() => {
    const handlerE = () => {};
    window.addEventListener("resize", handlerE, { capture: true });
    return () => window.removeEventListener("resize", handlerE, true);
  }, []);
  return null;
}

function F() {
  useEffect(() => {
    const controller = new AbortController();
    window.addEventListener("resize", () => {}, { signal: controller.signal });
    return () => controller.abort();
  }, []);
  return null;
}

function G() {
  useEffect(() => {
    const { signal } = new AbortController();
    window.addEventListener("resize", () => {}, { signal });
  }, []);
  return null;
}

function H() {
  const ref = useRef(null);
  useEffect(() => {
    const el = ref.current;
    const handlerH = () => {};
    el.addEventListener("click", handlerH);
    return () => el.removeEventListener("click", handlerH);
  }, []);
  return null;
}

function I() {
  useEffect(() => {
    const handlerI = () => {};
    document.addEventListener("click", handlerI);
    return () => window.removeEventListener("click", handlerI);
  }, []);
  return null;
}

function J() {
  useEffect(() => {
    const handlerJ = () => {};
    addEventListener("click", handlerJ);
    return () => window.removeEventListener("click", handlerJ);
  }, []);
  return null;
}

function K() {
  useEffect(() => {
    const handlerK = () => {};
    const type = "click";
    window.addEventListener(type, handlerK);
    function cleanup() {
      window.removeEventListener("click", handlerK);
    }
    return () => cleanup();
  }, []);
  return null;
}

function L() {
  useEffect(() => {
    const handlerL = () => {};
    window.addEventListener("click", handlerL);
    const cleanup = () => {
      window.removeEventListener("click", handlerL);
    };
    return cleanup;
  }, []);
  return null;
}

function M() {
  useEffect(() => {
    const handlerM = () => {};
    window.addEventListener("click", handlerM);
    window.removeEventListener("click", handlerM);
  }, []);
  return null;
}

function N() {
  window.addEventListener("click", () => {});
  useLayoutEffect(() => {
    window.addEventListener("keydown", onKeyN);
    return () => {
      window.removeEventListener("keyup", onKeyN);
    };
  });
  useEffect(() => {
    const opts = { capture: true };
    window.addEventListener("click", onKeyN, opts);
    return () => window.removeEventListener("click", onKeyN, opts);
  });
  useEffect(() => {
    window.removeEventListener("click", () => {});
  });
  return null;
}

function O({ signal }) {
  useEffect(() => {
    window.addEventListener("click", () => {}, { signal });
  }, []);
  return null;
}

function P() {
  useEffect(() => {
    const handlers = { cleanup() { window.removeEventListener("click", onKeyP); } };
    window.addEventListener("click", onKeyP);
    return () => handlers.cleanup();
  }, []);
  return null;
}
