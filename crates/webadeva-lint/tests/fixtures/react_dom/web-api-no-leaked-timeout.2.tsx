function A() {
  const timer = useRef<number>();
  useEffect(() => {
    timer.current = window.setTimeout(() => {}, 1) as unknown as number;
    return () => {
      if (timer.current) clearTimeout(timer.current!);
    };
  }, []);
  useEffect(() => {
    const t = setTimeout(() => {}, 1);
    return cleanup;
  });
  useEffect(() => {
    const t2 = setTimeout(() => {}, 1);
  });
  useEffect(() => () => clearTimeout(t2));
  useEffect(() => {
    const [a, b] = [setTimeout(() => {}, 1), 2];
    const { c } = { c: setTimeout(() => {}, 1) };
    return () => { clearTimeout(a); clearTimeout(c); };
  });
  useEffect(() => {
    for (const x of xs) {
      const t3 = setTimeout(() => {}, 1);
    }
    return () => { for (const t3 of ts) clearTimeout(t3); };
  });
  useEffect((() => {
    const t4 = setTimeout(() => {}, 1);
  }) as any);
  return null;
}
class K extends React.Component {
  timer = setTimeout(() => {}, 1);
  componentDidMount() {
    this.t = setTimeout(() => {}, 1);
  }
}
