async function drawGraph(baseUrl, isHome, pathColors, graphConfig, targetContainer) {

  let {
  depth,
  enableDrag,
  enableLegend,
  enableZoom,
  opacityScale,
  scale,
  repelForce,
  linkDistance,
  fontSize} = graphConfig;

  const container = targetContainer || document.getElementById("graph-container")
  const { index, links, content } = await fetchData

  // Use .pathname to remove hashes / searchParams / text fragments
  const cleanUrl = window.location.origin + window.location.pathname

  const curPage = cleanUrl.replace(/\/$/g, "").replace(baseUrl, "")

  const parseIdsFromLinks = (links) => [
    ...new Set(links.flatMap((link) => [link.source, link.target])),
  ]

  // Links is mutated by d3. We want to use links later on, so we make a copy and pass that one to d3
  // Note: shallow cloning does not work because it copies over references from the original array
  const copyLinks = JSON.parse(JSON.stringify(links))

  const neighbours = new Set()
  const wl = [curPage || "/", "__SENTINEL"]
  if (depth >= 0) {
    while (depth >= 0 && wl.length > 0) {
      // compute neighbours
      const cur = wl.shift()
      if (cur === "__SENTINEL") {
        depth--
        wl.push("__SENTINEL")
      } else {
        neighbours.add(cur)
        const outgoing = index.links[cur] || []
        const incoming = index.backlinks[cur] || []
        wl.push(...outgoing.map((l) => l.target), ...incoming.map((l) => l.source))
      }
    }
  } else {
    parseIdsFromLinks(copyLinks).forEach((id) => neighbours.add(id))
  }

  const data = {
    nodes: [...neighbours].map((id) => ({ id })),
    links: copyLinks.filter((l) => neighbours.has(l.source) && neighbours.has(l.target)),
  }

  const color = (d) => {
    if (d.id === curPage || (d.id === "/" && curPage === "")) {
      return "var(--g-node-active)"
    }

    for (const pathColor of pathColors) {
      const path = Object.keys(pathColor)[0]
      const colour = pathColor[path]
      if (d.id.startsWith(path)) {
        return colour
      }
    }

    return "var(--g-node)"
  }

  const drag = (simulation) => {
    function dragstarted(event, d) {
      if (!event.active) simulation.alphaTarget(1).restart()
      d.fx = d.x
      d.fy = d.y
    }

    function dragged(event, d) {
      d.fx = event.x
      d.fy = event.y
    }

    function dragended(event, d) {
      if (!event.active) simulation.alphaTarget(0)
      d.fx = null
      d.fy = null
    }

    const noop = () => {}
    return d3
      .drag()
      .on("start", enableDrag ? dragstarted : noop)
      .on("drag", enableDrag ? dragged : noop)
      .on("end", enableDrag ? dragended : noop)
  }

  const height = Math.max(container.offsetHeight, isHome ? 500 : 250)
  const width = container.offsetWidth

  const simulation = d3
    .forceSimulation(data.nodes)
    .force("charge", d3.forceManyBody().strength(-100 * repelForce))
    .force(
      "link",
      d3
        .forceLink(data.links)
        .id((d) => d.id)
        .distance(40 * linkDistance),
    )
    .force("center", d3.forceCenter())
    .force("collide", d3.forceCollide().radius(12).strength(0.7))

  const svg = d3
    .select(container)
    .append("svg")
    .attr("width", width)
    .attr("height", height)
    .attr('viewBox', [-width / 2 * 1 / scale, -height / 2 * 1 / scale, width * 1 / scale, height * 1 / scale])

  const isExternalId = (id) => id.startsWith("/blog/") || id.startsWith("/book/")
  const endpointIds = (l) => [
    typeof l.source === "string" ? l.source : l.source.id,
    typeof l.target === "string" ? l.target : l.target.id,
  ]
  const isBrainToExternal = (l) => {
    const [s, t] = endpointIds(l)
    return !isExternalId(s) && isExternalId(t)
  }
  const isCrossSite = (l) => {
    const [s, t] = endpointIds(l)
    return isExternalId(s) !== isExternalId(t)
  }

  if (enableLegend) {
    const pathLegendLabels = { "/blog/": "Blog", "/book/": "Book" }
    const presentPaths = pathColors.filter((pc) => {
      const path = Object.keys(pc)[0]
      return data.nodes.some((n) => n.id && n.id.startsWith(path))
    })
    const entries = [
      { type: "dot", color: "var(--g-node-active)", label: "Current" },
      { type: "dot", color: "var(--g-node)", label: "Note" },
      ...presentPaths.map((pc) => {
        const path = Object.keys(pc)[0]
        return { type: "dot", color: pc[path], label: pathLegendLabels[path] || path }
      }),
    ]
    const hasOutgoing = data.links.some(isBrainToExternal)
    if (hasOutgoing) {
      entries.push({ type: "line", stroke: "var(--g-link)", dasharray: "2,3", label: "Outgoing" })
    }
    const legendX = -width / (2 * scale) + 14
    const legendBaseY = height / (2 * scale) - 8
    const legendStep = 15
    entries.forEach((entry, i) => {
      const y = legendBaseY - legendStep * (entries.length - 1 - i)
      if (entry.type === "dot") {
        svg
          .append("circle")
          .attr("cx", legendX)
          .attr("cy", y)
          .attr("r", 4)
          .style("fill", entry.color)
      } else {
        svg
          .append("line")
          .attr("x1", legendX - 6)
          .attr("y1", y)
          .attr("x2", legendX + 6)
          .attr("y2", y)
          .attr("stroke", entry.stroke)
          .attr("stroke-width", 1.5)
          .attr("stroke-dasharray", entry.dasharray)
      }
      svg
        .append("text")
        .attr("x", legendX + 11)
        .attr("y", y)
        .text(entry.label)
        .style("font-size", "9.5px")
        .style("fill", "var(--gray)")
        .attr("alignment-baseline", "middle")
    })
  }

  // draw links between nodes
  const link = svg
    .append("g")
    .selectAll("line")
    .data(data.links)
    .join("line")
    .attr("class", (d) => isCrossSite(d) ? "link link-cross" : "link")
    .attr("stroke", "var(--g-link)")
    .attr("stroke-width", 1.25)
    .attr("stroke-opacity", 0.45)
    .attr("stroke-dasharray", (d) => isBrainToExternal(d) ? "2,3" : null)
    .attr("data-source", (d) => d.source.id)
    .attr("data-target", (d) => d.target.id)

  // svg groups
  const graphNode = svg.append("g").selectAll("g").data(data.nodes).enter().append("g")

  // calculate radius
  const nodeRadius = (d) => {
    const numOut = index.links[d.id]?.length || 0
    const numIn = index.backlinks[d.id]?.length || 0
    return 2 + Math.sqrt(numOut + numIn)
  }

  // draw individual nodes
  const node = graphNode
    .append("circle")
    .attr("class", "node")
    .attr("id", (d) => d.id)
    .attr("r", nodeRadius)
    .attr("fill", color)
    .style("cursor", "pointer")
    .on("click", (_, d) => {
      if (d.id.startsWith("/blog/")) {
        window.open(`https://www.ssp.sh${d.id}/`, "_blank", "noopener")
        return
      }
      if (d.id.startsWith("/book/")) {
        window.open(`https://www.dedp.online${d.id.replace(/^\/book/, "")}.html`, "_blank", "noopener")
        return
      }
      // SPA navigation
      window.Million.navigate(new URL(`${baseUrl}${decodeURI(d.id).replace(/\s+/g, "-")}/`), ".singlePage")
    })
    .on("mouseover", function (_, d) {
      svg.selectAll(".node").transition().duration(100).attr("fill", "var(--g-node-inactive)")

      const neighbours = parseIdsFromLinks([
        ...(index.links[d.id] || []),
        ...(index.backlinks[d.id] || []),
      ])
      const neighbourNodes = svg.selectAll(".node").filter((d) => neighbours.includes(d.id))
      const currentId = d.id
      if (!d.id.startsWith("/blog/") && !d.id.startsWith("/book/")) {
        window.Million.prefetch(new URL(`${baseUrl}${decodeURI(d.id).replace(/\s+/g, "-")}/`))
      }
      const linkNodes = svg
        .selectAll(".link")
        .filter((d) => d.source.id === currentId || d.target.id === currentId)

      // highlight neighbour nodes
      neighbourNodes.transition().duration(200).attr("fill", color)

      // highlight links
      linkNodes.transition().duration(200).attr("stroke", "var(--g-link-active)")

      const bigFont = fontSize*1.5

      // show text for self
      d3.select(this.parentNode)
        .raise()
        .select("text")
        .transition()
        .duration(200)
        .attr('opacityOld', d3.select(this.parentNode).select('text').style("opacity"))
        .style('opacity', 1)
        .style('font-size', bigFont+'em')
        .attr('dy', d => nodeRadius(d) + 20 + 'px') // radius is in px
    })
    .on("mouseleave", function (_, d) {
      svg.selectAll(".node").transition().duration(200).attr("fill", color)

      const currentId = d.id
      const linkNodes = svg
        .selectAll(".link")
        .filter((d) => d.source.id === currentId || d.target.id === currentId)

      linkNodes.transition().duration(200).attr("stroke", "var(--g-link)")

      d3.select(this.parentNode)
      .select("text")
      .transition()
      .duration(200)
      .style('opacity', d3.select(this.parentNode).select('text').attr("opacityOld"))
      .style('font-size', fontSize+'em')
      .attr('dy', d => nodeRadius(d) + 8 + 'px') // radius is in px
    })
    .call(drag(simulation))

  // draw labels
  const labels = graphNode
    .append("text")
    .attr("dx", 0)
    .attr("dy", (d) => nodeRadius(d) + 8 + "px")
    .attr("text-anchor", "middle")
    .text((d) => content[d.id]?.title || d.id.replace("-", " "))
    .style('opacity', (opacityScale - 1) / 3.75)
    .style("fill", "currentColor")
    .style("pointer-events", "none")
    .style('font-size', fontSize+'em')
    .raise()
    .call(drag(simulation))

  // set panning

  if (enableZoom) {
    svg.call(
      d3
        .zoom()
        .extent([
          [0, 0],
          [width, height],
        ])
        .scaleExtent([0.25, 4])
        .on("zoom", ({ transform }) => {
          link.attr("transform", transform)
          node.attr("transform", transform)
          const scale = transform.k * opacityScale;
          const scaledOpacity = Math.max((scale - 1) / 3.75, 0)
          labels.attr("transform", transform).style("opacity", scaledOpacity)
        }),
    )
  }

  // progress the simulation
  simulation.on("tick", () => {
    link
      .attr("x1", (d) => d.source.x)
      .attr("y1", (d) => d.source.y)
      .attr("x2", (d) => d.target.x)
      .attr("y2", (d) => d.target.y)
    node.attr("cx", (d) => d.x).attr("cy", (d) => d.y)
    labels.attr("x", (d) => d.x).attr("y", (d) => d.y)
  })

  return simulation
}
