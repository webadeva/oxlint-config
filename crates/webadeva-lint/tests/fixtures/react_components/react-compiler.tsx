import { useForm } from "react-hook-form";
import RHF, * as all from "react-hook-form";
import { useReactTable } from "@tanstack/react-table";

function Component() {
  "use memo if(isEnabled)";
  "use memo if(not valid)";
  const form = useForm();
  return <div>{String(form)}</div>;
}

function Other() {
  "use no memo";
  return <span />;
}
