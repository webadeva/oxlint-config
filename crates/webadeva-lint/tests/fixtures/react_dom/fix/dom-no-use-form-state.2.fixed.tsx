import { useActionState } from "preact/compat";
import { useFormState } from "react-dom";
export const useIt = () => useActionState(fn, 0);
