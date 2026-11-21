import { useActionState } from "react";
import { useActionState } from "react";
import { useActionState } from "react";
import { useActionState } from "react";
import { useActionState } from "react";
import { useFormState } from "react-dom";
import * as ReactDOM from "react-dom";

export function Form() {
  const [state, action] = useActionState(submit, null);
  const [s2, a2] = useActionState(submit, 0);
  const [s3] = (useActionState)(submit, 1);
  const [s4] = useActionState<number>(submit, 1);
  const [s5] = (useActionState)(submit, 1);
  return <form action={action}>{state}{s2}{a2}{s3}{s4}{s5}</form>;
}
