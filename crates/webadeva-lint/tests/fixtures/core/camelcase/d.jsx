import Comp_name from "x";
const el = <Comp_name prop_x={1}><Comp_name.Sub_x /></Comp_name>;
const el2 = <Unresolved_comp>text</Unresolved_comp>;
const el3 = <div data_attr="x" />;
function Local_comp() { return null; }
const el4 = <Local_comp />;
