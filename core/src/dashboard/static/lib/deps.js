import React from "https://esm.sh/react@18.3.1?dev";
import ReactDOM from "https://esm.sh/react-dom@18.3.1/client?dev";
import htm from "https://esm.sh/htm@3.1.1?dev";
import * as d3 from "https://esm.sh/d3@7.9.0?dev";

const html = htm.bind(React.createElement);

export { React, ReactDOM, html, d3 };
