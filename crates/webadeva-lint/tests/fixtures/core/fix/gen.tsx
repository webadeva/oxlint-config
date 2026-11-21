function identity<T>(x: T): T {
  return x;
}
const pair = function <A, B>(a: A, b: B) {
  return [a, b];
};
const tc = function <T,>(x: T) {
  return x;
};
const Comp = (props: Props) => <div>{props.children}</div>;
const make = <T extends object>(x: T) => x;
