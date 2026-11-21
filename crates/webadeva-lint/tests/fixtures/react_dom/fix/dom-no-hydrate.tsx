import { hydrate } from "react-dom";
import ReactDOM from "react-dom";
import * as Dom from "react-dom";

hydrate(<App />, document.getElementById("root"));
ReactDOM.hydrate(<App />, root, () => console.log("done"));
Dom.hydrate((<App />), el!);
hydrate(<App />);
ReactDOM?.hydrate(<App />, other);
