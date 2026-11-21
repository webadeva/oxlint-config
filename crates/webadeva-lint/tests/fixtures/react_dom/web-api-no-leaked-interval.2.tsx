function A() {
  const timer = useRef<number>();
  useEffect(() => {
    timer.current = window.setInterval(() => {}, 1) as unknown as number;
    return () => {
      if (timer.current) clearInterval(timer.current!);
    };
  }, []);
  useEffect(() => {
    const t = setInterval(() => {}, 1);
    return cleanup;
  });
  useEffect(() => {
    const t2 = setInterval(() => {}, 1);
  });
  useEffect(() => () => clearInterval(t2));
  useEffect(() => {
    const [a, b] = [setInterval(() => {}, 1), 2];
    const { c } = { c: setInterval(() => {}, 1) };
    return () => { clearInterval(a); clearInterval(c); };
  });
  useEffect(() => {
    for (const x of xs) {
      const t3 = setInterval(() => {}, 1);
    }
    return () => { for (const t3 of ts) clearInterval(t3); };
  });
  useEffect((() => {
    const t4 = setInterval(() => {}, 1);
  }) as any);
  return null;
}
class K extends React.Component {
  timer = setInterval(() => {}, 1);
  componentDidMount() {
    this.t = setInterval(() => {}, 1);
  }
}
