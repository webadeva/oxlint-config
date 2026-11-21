import { useEffect } from "react";

function A() {
  useEffect(() => {
    fetch("/api");
  }, []);
  return null;
}

function B() {
  useEffect(() => {
    const controller = new AbortController();
    fetch("/api", { signal: controller.signal });
    return () => controller.abort();
  }, []);
  return null;
}

function C() {
  useEffect(() => {
    const controller = new AbortController();
    fetch("/api", { signal: controller.signal });
  }, []);
  return null;
}

function D() {
  useEffect(() => {
    const controller = new AbortController();
    const signal = controller.signal;
    const options = { signal };
    fetch("/api", options);
    return () => {
      controller.abort();
    };
  }, []);
  return null;
}

function E({ signal }) {
  useEffect(() => {
    fetch("/api", { signal });
  }, []);
  return null;
}

function F() {
  useEffect(() => {
    const load = async () => {
      await fetch("/api");
    };
    load();
  }, []);
  fetch("/x");
  return null;
}

function G() {
  useEffect(() => {
    const { signal } = new AbortController();
    fetch("/api", { signal });
  }, []);
  return null;
}

function H() {
  useEffect(() => {
    const c1 = new AbortController();
    const c2 = new AbortController();
    api.fetch("/api", { signal: c1.signal });
    return () => c2.abort();
  }, []);
  return null;
}

function I() {
  useEffect(() => {
    const controller = new AbortController();
    fetch("/api", { signal: controller.signal });
    controller.abort();
  }, []);
  return null;
}
