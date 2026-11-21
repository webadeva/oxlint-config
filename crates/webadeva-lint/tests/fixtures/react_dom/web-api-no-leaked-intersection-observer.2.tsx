function A() {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const ro = new IntersectionObserver((entries) => {
      ro.disconnect();
    });
    ro.observe(ref.current!);
  }, []);
  useEffect(() => {
    let ro2: IntersectionObserver;
    ro2 = new IntersectionObserver(() => {});
    ro2.observe(document.body);
    return () => ro2.disconnect();
  }, []);
  useEffect(() => {
    const ro3 = new IntersectionObserver(() => {});
    els.map((e) => ro3.observe(e));
    return () => ro3.disconnect();
  }, []);
  useEffect(() => {
    const ro4 = new IntersectionObserver(() => {}) as IntersectionObserver;
    ro4.observe(a);
    ro4.observe(b);
    return () => {
      ro4.unobserve(a);
    };
  }, []);
  useLayoutEffect(() => {
    const ro5 = new window.IntersectionObserver(() => {});
    ro5.observe(a);
  });
  return null;
}
