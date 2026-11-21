import React, { cloneElement, Children } from "react";
function A({ items }) {
  return (
    <>
      {items.map((item, index) => <li key={index}>{item}</li>)}
      {items.map((item, i) => <li key={`k-${i}`}>{item}</li>)}
      {items.map((item, i) => <li key={"k" + i}>{item}</li>)}
      {items.map((item, i) => <li key={i.toString()}>{item}</li>)}
      {items.map((item, i) => <li key={String(i)}>{item}</li>)}
      {items.forEach((item, i) => <li key={i} />)}
      {items.filter((item, i) => i).map((item) => <li key={item.id} />)}
      {Children.map(children, (child, i) => <li key={i} />)}
      {items.map((item, i) => React.createElement("li", { key: i }))}
      {items.map((item, i) => cloneElement(item, { key: i }))}
      {Array.from(items, (item, i) => <li key={i} />)}
      {items.flatMap((item, i) => [<li key={i} />])}
      {items.reduce((acc, item, i) => [...acc, <li key={i} />], [])}
      {items.map(({ id }, i) => <li key={id + i} />)}
      {items.findIndex((item, i) => <li key={i} />)}
      {items.some((item, i) => <li key={i} />)}
    </>
  );
}
