import { React, ReactDOM, html } from "./lib/deps.js";
import DashboardApp from "./components/DashboardApp.js";

const rootEl = document.getElementById("root");
const root = ReactDOM.createRoot(rootEl);
root.render(html`<${DashboardApp} />`);
