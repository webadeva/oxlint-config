import { render } from "react-dom";
import * as RD from "react-dom";
const a = render(<App />, root);
const b = ReactDOM.render(<App />, root);
const c = RD.render(<App />, root);
render(<App />, root);
ReactDOM.render(<App />, root);
function f() {
  return ReactDOM.render(<App />, root);
}
const g = () => render(<App />, root);
let h;
h = render(<App />, root);
const o = { inst: render(<App />, root) };
const i = render(<App />, root) as any;
const j = root.render(<App />);
const k = (render(<App />, root));
call(render(<App />, root));
