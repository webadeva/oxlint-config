#!/usr/bin/env node

/** docs */
import { hydrateRoot } from "react-dom/client";
import { hydrate } from "react-dom";
hydrateRoot(document.getElementById("root"), <App />);
