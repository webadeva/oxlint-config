import { createRoot } from "react-dom/client";
import { hydrateRoot } from "react-dom/client";
import { createRoot } from "react-dom/client";
import { useActionState } from "react";
import { render, hydrate, useFormState } from "react-dom";

function Form() {
  const [s, act] = useActionState(go, null);
  return <form className="f" action={act}><label htmlFor="n">{s}</label></form>;
}

createRoot(document.getElementById("a")).render(<Form />);
hydrateRoot(document.getElementById("b"), <div className="z" />);
createRoot(el).render(<p tabIndex={1} />);
