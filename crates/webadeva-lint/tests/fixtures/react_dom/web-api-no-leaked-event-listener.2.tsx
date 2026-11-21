function A() {
  useEffect(() => {
    const onA = () => {};
    window.addEventListener("click", onA);
    function cleanup() {
      window.removeEventListener("click", onA);
    }
    return () => cleanup();
  }, []);
  useEffect(() => {
    const onB = () => {};
    window.addEventListener("click", onB);
    const helpers = { stop: () => window.removeEventListener("click", onB) };
    return () => helpers.stop();
  }, []);
  useEffect(() => {
    const onC = () => {};
    window.addEventListener("click", onC);
    return () => {
      window.removeEventListener("click", onC, { capture: false });
    };
  }, []);
  useEffect(() => {
    const onD = () => {};
    const capture = true;
    window.addEventListener("click", onD, { capture });
    return () => window.removeEventListener("click", onD, { capture: true });
  }, []);
  useEffect(() => {
    const onE = () => {};
    el.addEventListener("click", onE);
    return () => other.removeEventListener("click", onE);
  }, []);
  useEffect(() => {
    const onF = () => {};
    rn.addEventListener("click", onF);
    BackHandler.addEventListener("hardwareBackPress", onF);
  }, []);
  useEffect(() => {
    const ctrl = new AbortController();
    window.addEventListener("click", () => {}, { signal: ctrl.signal, capture: true });
    const opts = { signal: ctrl.signal };
    window.addEventListener("keyup", () => {}, opts);
    return () => ctrl.abort();
  }, []);
  useEffect(() => {
    const onG = () => {};
    window.addEventListener(`click`, onG);
    return () => window.removeEventListener("click", onG);
  }, []);
  return null;
}
