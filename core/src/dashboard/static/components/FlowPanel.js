import { React, html } from "../lib/deps.js";
import { renderFlowGraph, tearDownFlowGraph } from "../lib/flowGraph.js";

export default function FlowPanel({
  flowData,
  flowSummary,
  formattedFlowUpdatedAt,
}) {
  const canvasRef = React.useRef(null);

  React.useEffect(() => {
    const canvas = canvasRef.current;
    if (canvas) {
      renderFlowGraph(canvas, flowData);
    }
  }, [flowData]);

  React.useEffect(() => {
    const handleResize = () => {
      const canvas = canvasRef.current;
      if (canvas) {
        renderFlowGraph(canvas, flowData);
      }
    };
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, [flowData]);

  React.useEffect(() => () => tearDownFlowGraph(), []);

  const hasData = flowSummary.nodes > 0 && flowSummary.flows > 0;

  return html`
    <section class="card flow-card">
      <div class="flow-header">
        <div>
          <h2>Event flow</h2>
          <p class="card-subtitle">Last updated ${formattedFlowUpdatedAt}</p>
        </div>
        <div class="flow-summary">
          <div class="summary-chip">
            <span class="summary-label">Nodes</span>
            <span class="summary-value">${flowSummary.nodes}</span>
          </div>
          <div class="summary-chip">
            <span class="summary-label">Flows</span>
            <span class="summary-value">${flowSummary.flows}</span>
          </div>
          <div class="summary-chip">
            <span class="summary-label">Active</span>
            <span class="summary-value">${flowSummary.activeLinks}</span>
          </div>
          <div class="summary-chip">
            <span class="summary-label">Topics</span>
            <span class="summary-value">${flowSummary.topics}</span>
          </div>
        </div>
      </div>
      <div class="flow-canvas" ref=${canvasRef}>
        ${hasData
          ? null
          : html`<div class="empty-state">Waiting for flow data...</div>`}
      </div>
      <footer class="legend">
        <span class="legend-item"
          ><span class="legend-dot agent"></span>Agent</span
        >
        <span class="legend-item"
          ><span class="legend-dot eventbus"></span>EventBus</span
        >
        <span class="legend-item"
          ><span class="legend-dot router"></span>Router</span
        >
        <span class="legend-item"
          ><span class="legend-dot llm"></span>LLM</span
        >
        <span class="legend-item"
          ><span class="legend-dot tool"></span>Tool</span
        >
        <span class="legend-item"
          ><span class="legend-dot storage"></span>Storage</span
        >
      </footer>
    </section>
  `;
}
