import { render, hydrate, useFormState } from "react-dom";

function Form() {
  const [s, act] = useFormState(go, null);
  return <form class="f" action={act}><label for="n">{s}</label></form>;
}

render(<Form />, document.getElementById("a"));
hydrate(<div class="z" />, document.getElementById("b"));
render(<p tabindex={1} />, el);
