// Entry point
/* block comment */
import { render } from "react-dom";
import ReactDOM from "react-dom";
import * as RD from "react-dom";

render(<App />, document.getElementById("root"));
ReactDOM.render(<App name="a" />, root);
RD.render((<App />), (el as HTMLElement));
render(<App />);
render(...args);
render(...args, el);
(render)(<App />, el, () => {});
const r = ReactDOM.render(<App />, el);
function mount() {
  return render(
    <Provider>
      <App />
    </Provider>,
    container,
  );
}
