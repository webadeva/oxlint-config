import { useFormState } from "react-dom";
import * as ReactDOM from "react-dom";

export function Form() {
  const [state, action] = useFormState(submit, null);
  const [s2, a2] = ReactDOM.useFormState(submit, 0);
  const [s3] = (useFormState)(submit, 1);
  const [s4] = useFormState<number>(submit, 1);
  const [s5] = (useFormState as any)(submit, 1);
  return <form action={action}>{state}{s2}{a2}{s3}{s4}{s5}</form>;
}
