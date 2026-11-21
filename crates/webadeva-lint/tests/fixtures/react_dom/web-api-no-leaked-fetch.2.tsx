function A() {
  useEffect(() => {
    const abortController = new AbortController();
    const run = async () => {
      const res = await fetch("/a", { signal: abortController.signal });
    };
    fetch("/b", { method: "GET", ...{ signal: abortController.signal } });
    return () => abortController.abort();
  }, []);
  useEffect(() => {
    const c = new AbortController();
    window.fetch("/c", { signal: c.signal });
    return () => c?.abort();
  }, []);
  useEffect(() => {
    fetch("/d", opts);
  }, []);
  useEffect(() => () => {
    fetch("/e");
  }, []);
  return null;
}
