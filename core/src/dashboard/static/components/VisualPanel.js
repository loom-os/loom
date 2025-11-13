import { html } from "../lib/deps.js";
import FlowPanel from "./FlowPanel.js";

export default function VisualPanel({
  flowData,
  flowSummary,
  formattedFlowUpdatedAt,
}) {
  return html`
    <section class="visual-panel">
      <${FlowPanel}
        flowData=${flowData}
        flowSummary=${flowSummary}
        formattedFlowUpdatedAt=${formattedFlowUpdatedAt}
      />
    </section>
  `;
}
