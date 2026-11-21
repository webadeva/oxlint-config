import { useEffect, useRef } from "react";

function A() {
  useEffect(() => {
    new IntersectionObserver(() => {});
  }, []);
  return null;
}

function B() {
  const ref = useRef(null);
  useEffect(() => {
    const observer = new IntersectionObserver(() => {});
    observer.observe(ref.current);
  }, []);
  return null;
}

function C() {
  const ref = useRef(null);
  useEffect(() => {
    const observer = new IntersectionObserver(() => {});
    observer.observe(ref.current);
    return () => observer.disconnect();
  }, []);
  return null;
}

function D() {
  const ref = useRef(null);
  useEffect(() => {
    const el = ref.current;
    const observer = new IntersectionObserver(() => {});
    observer.observe(el);
    return () => {
      observer.unobserve(el);
    };
  }, []);
  return null;
}

function E({ items }) {
  useEffect(() => {
    const observer = new IntersectionObserver(() => {});
    items.forEach((item) => observer.observe(item));
    return () => {
      items.forEach((item) => observer.unobserve(item));
    };
  }, []);
  return null;
}

function F() {
  const ref = useRef(null);
  useEffect(() => {
    const observer = new IntersectionObserver(() => {});
    if (ref.current) observer.observe(ref.current);
  }, []);
  return null;
}

function G() {
  const observerRef = useRef(new IntersectionObserver(() => {}));
  useEffect(() => {
    const observer = observerRef.current;
    observer.observe(el);
    return () => observerRef.current.disconnect();
  }, []);
  return null;
}

function H() {
  const observerRef = useRef(new IntersectionObserver(() => {}));
  useEffect(() => {
    observerRef.current.observe(el);
  }, []);
  return null;
}

function I() {
  useEffect(() => {
    const observer = new IntersectionObserver(() => {});
    observer.observe(el);
    function cleanup() {
      observer.disconnect();
    }
    return cleanup;
  }, []);
  return null;
}

function J() {
  useEffect(() => {
    const observer = new IntersectionObserver(() => {});
    observer.observe(el);
    const stop = () => observer.disconnect();
    return () => stop();
  }, []);
  return null;
}

function K() {
  const o = new IntersectionObserver(() => {});
  return null;
}
