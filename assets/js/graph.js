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

  let activeFilter = null
  const filterDim = 0.18

  const matchesFilter = (d) => {
    if (!activeFilter) return true
    if (activeFilter === "note") return !d.id.startsWith("/blog/") && !d.id.startsWith("/book/")
    return d.id.startsWith(activeFilter)
  }

  const linkMatchesFilter = (l) => {
    if (!activeFilter) return true
    const sId = typeof l.source === "string" ? l.source : l.source.id
    const tId = typeof l.target === "string" ? l.target : l.target.id
    return matchesFilter({ id: sId }) && matchesFilter({ id: tId })
  }

  const applyFilter = () => {
    graphNode.transition().duration(200).style("opacity", (d) => matchesFilter(d) ? 1 : filterDim)
    link.transition().duration(200).style("opacity", (l) => linkMatchesFilter(l) ? 1 : filterDim)
    svg.selectAll(".legend-entry").each(function () {
      const f = d3.select(this).attr("data-filter")
      const isActive = f && f === activeFilter
      d3.select(this).select("circle")
        .attr("stroke", isActive ? "var(--g-node-active)" : null)
        .attr("stroke-width", isActive ? 1.5 : null)
    })
  }

  if (enableLegend) {
    const pathLegendLabels = { "/blog/": "Blog", "/book/": "Book" }
    const presentPaths = pathColors.filter((pc) => {
      const path = Object.keys(pc)[0]
      return data.nodes.some((n) => n.id && n.id.startsWith(path))
    })
    const entries = [
      { color: "var(--g-node-active)", label: "Current" },
      { color: "var(--g-node)", label: "Note", filter: "note" },
      ...presentPaths.map((pc) => {
        const path = Object.keys(pc)[0]
        return { color: pc[path], label: pathLegendLabels[path] || path, filter: path }
      }),
    ]
    const legendX = -width / (2 * scale) + 14
    const legendBaseY = height / (2 * scale) - 8
    const legendStep = 15
    entries.forEach((entry, i) => {
      const y = legendBaseY - legendStep * (entries.length - 1 - i)
      const g = svg
        .append("g")
        .attr("class", "legend-entry")
        .attr("data-filter", entry.filter || "")
        .style("cursor", entry.filter ? "pointer" : "default")
      g.append("circle")
        .attr("cx", legendX)
        .attr("cy", y)
        .attr("r", 4)
        .style("fill", entry.color)
      g.append("text")
        .attr("x", legendX + 11)
        .attr("y", y)
        .text(entry.label)
        .style("font-size", "9.5px")
        .style("fill", "var(--gray)")
        .attr("alignment-baseline", "middle")
      if (entry.filter) {
        g.on("click", (event) => {
          event.stopPropagation()
          activeFilter = activeFilter === entry.filter ? null : entry.filter
          applyFilter()
        })
      }
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
        window.closeLightbox?.()
        return
      }
      if (d.id.startsWith("/book/")) {
        window.open(`https://www.dedp.online${d.id.replace(/^\/book/, "")}.html`, "_blank", "noopener")
        window.closeLightbox?.()
        return
      }
      // SPA navigation
      window.Million.navigate(new URL(`${baseUrl}${decodeURI(d.id).replace(/\s+/g, "-")}/`), ".singlePage")
      window.closeLightbox?.()
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
