function hotline() {
  clear_and_create("hotline");

  function getOrCreateElement(id, className, type = "div") {
    let element = document.getElementById(id);
    if (!element) {
      element = document.createElement(type);
      element.id = id;
      if (className) element.className = className;
      document.body.appendChild(element);
    }
    return element;
  }

  const hotlineDiv = getOrCreateElement("hotline", "tabcontent");
  const hotlineRunsDiv = getOrCreateElement("hotline-runs", "extra");
  if (!hotlineDiv.contains(hotlineRunsDiv)) {
    hotlineDiv.appendChild(hotlineRunsDiv);
  }

  let h1 = hotlineRunsDiv.querySelector("h1");
  if (!h1) {
    h1 = document.createElement("h1");
    h1.textContent = "Hotline";
    hotlineRunsDiv.insertBefore(h1, hotlineRunsDiv.firstChild);
  }

  const buttonContainer = getOrCreateElement("button-container", "", "div");
  const tableContent = getOrCreateElement("table-content", "", "div");
  if (!hotlineRunsDiv.contains(buttonContainer)) {
    hotlineRunsDiv.appendChild(buttonContainer);
  }
  if (!hotlineRunsDiv.contains(tableContent)) {
    hotlineRunsDiv.appendChild(tableContent);
  }

  buttonContainer.innerHTML = "";
  tableContent.innerHTML = "";
  const memoryConfigs = [
    {
      id: "completion_node",
      title: "Execution Latency by Completion Node",
      description:
        "Bins memory operations by completion node (L1/L2/L3/DRAM) as reported by SPE. The L1% / L2% / L3% / DRAM% columns are the total percentage of packets that were identified with the respective completion node. Each completion node is further divided into a sub-histogram, binning latencies by cutoffs determined from the lat_mem_rd microbenchmark.",
    },
    {
      id: "execution_latency",
      title: "Execution Latency",
      description:
        "Shows hot memory operations by execution latency in nanoseconds, computed as total_latency - issue_latency - translation_latency. Total count is total number of samples processed, and dropped packets are the number of packets that saturated (exceeded SPE counters), and were dropped from the aggregated data.",
    },
    {
      id: "issue_latency",
      title: "Issue Latency",
      description:
        "Shows hot memory operations by issue latency in nanoseconds. Total count is total number of samples processed, and dropped packets are the number of packets that saturated (exceeded SPE counters), and were dropped from the aggregated data.",
    },
    {
      id: "translation_latency",
      title: "MMU Translation Latency",
      description:
        "Shows hot memory operations by MMU translation latency in nanoseconds. Total count is total number of samples processed, and dropped packets are the number of packets that saturated (exceeded SPE counters), and were dropped from the aggregated data.",
    },
  ];
  const branchConfigs = [
    {
      id: "branch",
      title: "Branch Misses",
      description:
        "Shows hot branch instructions by count and misprediction rate.",
    },
  ];

  const memoryGroup = document.createElement("div");
  memoryGroup.className = "button-group";
  const memoryLabel = document.createElement("h3");
  memoryLabel.textContent = "Memory Operations";
  memoryGroup.appendChild(memoryLabel);
  memoryConfigs.forEach((config) => {
    const button = document.createElement("button");
    button.textContent = config.title;
    button.className = "table-button";
    button.addEventListener("click", () => {
      document
        .querySelectorAll(".table-button")
        .forEach((btn) => btn.classList.remove("active"));
      button.classList.add("active");
      loadTables(config);
    });
    memoryGroup.appendChild(button);
  });

  const branchGroup = document.createElement("div");
  branchGroup.className = "button-group";
  const branchLabel = document.createElement("h3");
  branchLabel.textContent = "Branch Predictor";
  branchGroup.appendChild(branchLabel);
  branchConfigs.forEach((config) => {
    const button = document.createElement("button");
    button.textContent = config.title;
    button.className = "table-button";
    button.addEventListener("click", () => {
      document
        .querySelectorAll(".table-button")
        .forEach((btn) => btn.classList.remove("active"));
      button.classList.add("active");
      loadTables(config);
    });
    branchGroup.appendChild(button);
  });

  buttonContainer.appendChild(memoryGroup);
  buttonContainer.appendChild(branchGroup);

  const firstButton = buttonContainer.querySelector(".table-button");
  if (firstButton) {
    firstButton.classList.add("active");
    loadTables(memoryConfigs[0]);
  }
}
function loadTables(config) {
  const tableContent = document.getElementById("table-content");
  if (!tableContent) {
    console.error("Table content container not found");
    return;
  }
  let contentWrapper = `
        <h2>${config.title}</h2>
        ${
          config.description
            ? `<p class="table-description">${config.description}</p>`
            : ""
        }
        <div class="table-container" style="display: flex; justify-content: space-between;">
    `;

  let no_data_div = `
        <div style="
              width: ${100 / hotline_profile_raw_data.runs.length}%;
              padding: 0 10px;
          ">
              <div>No data collected</div>
          </div>
          `;

  if (
    !hotline_profile_raw_data ||
    !hotline_profile_raw_data.runs ||
    !hotline_profile_raw_data.runs.length
  ) {
    console.error("No data collected");
    contentWrapper += no_data_div;
  } else {
    hotline_profile_raw_data.runs.forEach((run, index) => {
      if (run.key_values.values === "No data collected") {
        contentWrapper += no_data_div;
      } else {
        const values = JSON.parse(run.key_values.values);

        contentWrapper += `
        <div style="width: ${
          100 / hotline_profile_raw_data.runs.length
        }%; padding: 0 10px;">
            <h3>${run.name}</h3>
            ${
              values.includes(config.id)
                ? `<iframe 
                    src="data/js/${run.name}_${config.id}.html" 
                    frameborder="0" 
                    style="width: 100%; height: 500px;"
                    onload="this.style.display='block'"
                    style="display: none">
                  </iframe>`
                : "<div>No data collected</div>"
            }
        </div>
    `;
      }
    });
  }

  contentWrapper += `</div>`;
  tableContent.innerHTML = contentWrapper;
}
