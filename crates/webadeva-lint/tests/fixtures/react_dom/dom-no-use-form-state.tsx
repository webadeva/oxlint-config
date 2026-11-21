import { useFormState } from "react-dom";
import * as RD from "react-dom";
const [s, a] = useFormState(action, 0);
const [s2, a2] = RD.useFormState(action, 0);
const [s3] = useActionState(action, 0);
