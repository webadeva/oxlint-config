const identity = <T,>(x: T): T => {
  return x;
};
const pair = <A, B>(a: A, b: B) => {
  return [a, b];
};
const tc = <T,>(x: T) => {
  return x;
};
const Comp = (props: Props) => { return <div>{props.children}</div> };
const make = <T extends object,>(x: T) => { return x };
