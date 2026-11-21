const identity = <T,>(x: T): T => x;
const pair = <A, B>(a: A, b: B) => [a, b];
const tc = <T,>(x: T) => x;
const Comp = (props: Props) => <div>{props.children}</div>;
const make = <T extends object>(x: T) => x;
