// Entry point
/* block comment */
import { createRoot } from "react-dom/client";
import { createRoot } from "react-dom/client";
import { createRoot } from "react-dom/client";
import { createRoot } from "react-dom/client";
import { createRoot } from "react-dom/client";
import { createRoot } from "react-dom/client";
import { createRoot } from "react-dom/client";
import { render } from "react-dom";
import ReactDOM from "react-dom";
import * as RD from "react-dom";

createRoot(document.getElementById("root")).render(<App />);
createRoot(root).render(<App name="a" />);
createRoot(el as HTMLElement).render(<App />);
render(<App />);
render(...args);
createRoot(el).render(...args);
createRoot(el).render(<App />);
const r = createRoot(el).render(<App />);
function mount() {
  return createRoot(container).render(<Provider>
      <App />
    </Provider>);
}
