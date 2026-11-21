async function ServerComp() {
  const x = Date.now();
  return <div>{x}</div>;
}

async function useAsyncHook() {
  return Date.now();
}

function Sync() {
  return <div>{Math.random()}</div>;
}
