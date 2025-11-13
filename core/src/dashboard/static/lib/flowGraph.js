import { d3 } from "./deps.js";

const nodeState = new Map();
let svg;
let linkGroup;
let nodeGroup;
let simulation;

export function tearDownFlowGraph() {
  if (svg) {
    svg.remove();
    svg = undefined;
    linkGroup = undefined;
    nodeGroup = undefined;
    simulation = undefined;
  }
  nodeState.clear();
}

export function renderFlowGraph(container, flowData) {
  if (!container) {
    return;
  }

  const hasData =
    flowData?.nodes?.length > 0 && flowData?.flows?.length > 0;
  if (!hasData) {
    tearDownFlowGraph();
    return;
  }

  const width = container.clientWidth;
  const height = container.clientHeight || 480;
  if (!width || !height) {
    return;
  }

  if (!svg) {
    svg = d3
      .select(container)
      .append("svg")
      .attr("class", "flow-svg")
      .attr("width", width)
      .attr("height", height);

    const defs = svg.append("defs");
    defs
      .append("marker")
      .attr("id", "arrowhead")
      .attr("viewBox", "0 -5 10 10")
      .attr("refX", 20)
      .attr("refY", 0)
      .attr("markerWidth", 6)
      .attr("markerHeight", 6)
      .attr("orient", "auto")
      .append("path")
      .attr("d", "M0,-5L10,0L0,5")
      .attr("fill", "#475569");

    linkGroup = svg.append("g").attr("class", "links");
    nodeGroup = svg.append("g").attr("class", "nodes");

    simulation = d3
      .forceSimulation()
      .force("link", d3.forceLink().id((d) => d.id).distance(140))
      .force("charge", d3.forceManyBody().strength(-360))
      .force("center", d3.forceCenter(width / 2, height / 2))
      .force("collision", d3.forceCollide().radius(42));
  } else {
    svg.attr("width", width).attr("height", height);
    simulation.force("center", d3.forceCenter(width / 2, height / 2));
  }

  const activeNodeIds = new Set();
  const nodes = flowData.nodes.map((node) => {
    const topics = Array.isArray(node.topics)
      ? node.topics
      : Array.from(node.topics || []);
    const cached = nodeState.get(node.id);
    if (cached) {
      cached.node_type = node.node_type;
      cached.event_count = node.event_count;
      cached.topics = topics;
      activeNodeIds.add(node.id);
      return cached;
    }
    const seed = {
      ...node,
      topics,
      x: width / 2 + (Math.random() - 0.5) * 40,
      y: height / 2 + (Math.random() - 0.5) * 40,
      vx: 0,
      vy: 0,
    };
    nodeState.set(node.id, seed);
    activeNodeIds.add(node.id);
    return seed;
  });

  nodeState.forEach((_, id) => {
    if (!activeNodeIds.has(id)) {
      nodeState.delete(id);
    }
  });

  const links = flowData.flows.map((flow) => ({
    ...flow,
    active: Date.now() - flow.last_event_ms < 4_000,
  }));

  const linkSelection = linkGroup
    .selectAll(".link")
    .data(links, (d) => `${d.source}-${d.target}-${d.topic}`);

  linkSelection.exit().remove();

  const linkEnter = linkSelection
    .enter()
    .append("path")
    .attr("class", "link")
    .attr("marker-end", "url(#arrowhead)");

  linkSelection
    .merge(linkEnter)
    .classed("active", (d) => d.active)
    .attr("stroke-dasharray", (d) => (d.active ? "6,4" : null));

  const nodeSelection = nodeGroup
    .selectAll(".node")
    .data(nodes, (d) => d.id);

  nodeSelection.exit().remove();

  const nodeEnter = nodeSelection
    .enter()
    .append("g")
    .attr("class", (d) => `node node-${d.node_type}`)
    .call(
      d3
        .drag()
        .on("start", (event, d) => {
          if (!event.active) {
            simulation.alphaTarget(0.3).restart();
          }
          d.fx = d.x;
          d.fy = d.y;
        })
        .on("drag", (event, d) => {
          d.fx = event.x;
          d.fy = event.y;
        })
        .on("end", (event, d) => {
          if (!event.active) {
            simulation.alphaTarget(0);
          }
          d.fx = null;
          d.fy = null;
        })
    );

  nodeEnter.append("circle").attr("r", 24);

  nodeEnter
    .append("text")
    .attr("class", "node-label")
    .attr("dy", 36)
    .text((d) => d.id);

  const badge = nodeEnter
    .append("g")
    .attr("class", "node-badge")
    .attr("transform", "translate(14,-16)");

  badge.append("circle").attr("r", 11);

  badge
    .append("text")
    .attr("class", "badge-text")
    .attr("dy", 4)
    .attr("text-anchor", "middle")
    .text((d) => (d.event_count > 99 ? "99+" : d.event_count));

  const nodeMerged = nodeSelection
    .merge(nodeEnter)
    .attr("class", (d) => `node node-${d.node_type}`);

  nodeMerged
    .select(".badge-text")
    .text((d) => (d.event_count > 99 ? "99+" : d.event_count));

  simulation.nodes(nodes);
  simulation.force("link").links(links);

  simulation.on("tick", () => {
    linkGroup.selectAll(".link").attr("d", (d) => {
      const source =
        typeof d.source === "object"
          ? d.source
          : nodes.find((n) => n.id === d.source);
      const target =
        typeof d.target === "object"
          ? d.target
          : nodes.find((n) => n.id === d.target);
      if (!source || !target) {
        return "";
      }
      const dx = target.x - source.x;
      const dy = target.y - source.y;
      const dr = Math.sqrt(dx * dx + dy * dy);
      return `M${source.x},${source.y}A${dr},${dr} 0 0,1 ${target.x},${target.y}`;
    });

    nodeGroup
      .selectAll(".node")
      .attr("transform", (d) => `translate(${d.x},${d.y})`);
  });

  simulation.alpha(0.12).restart();
}
