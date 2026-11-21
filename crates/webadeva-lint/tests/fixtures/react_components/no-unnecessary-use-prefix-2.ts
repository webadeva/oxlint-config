export const useExported = () => 1;
export const useCallsInCallback = () => {
  return useMemo(() => 1, []);
};
export function useOuter() {
  function inner() {
    return useState(0);
  }
  return inner;
}
const obj: any = {};
obj.useAssigned = function () {
  return 1;
};
obj.useAssigned2 = () => {
  useEffect(() => {});
};
obj["useComputed"] = () => 1;
const useTernary = cond ? () => 1 : () => 2;
const useTyped = (() => 1) as () => number;
function wrapper(useParam = () => 1) {
  return useParam;
}
const { useDestructured = () => 1 } = obj;
let useLet;
useLet = () => 1;
useLet += () => 1;
const use1Number = () => 1;
const usefulThing = () => 1;
const useA = function useB() {
  return 1;
};
const usePrivate = class {
  #useHidden() {
    return 1;
  }
  useMethod = () => 1;
};
function useCallsReactUse() {
  return React.use(ctx);
}
function useMember() {
  return foo.useBar();
}
function useOptional() {
  return foo?.useBar();
}
