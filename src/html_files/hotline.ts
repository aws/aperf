document.addEventListener('DOMContentLoaded', function() {
    const memoryConfigs = [
        {
            id: 'completion_node',
            title: 'Execution Latency by Completion Node',
            description: 'The completion node view bins the memory operations by completion node (L1/L2/L3/DRAM) as reported by SPE and execution latency. The L1 % / L2 % / L3 % / DRAM % columns show the total percentage of packets that were identified with the respective completion node. Each completion node is further divided into a sub histogram, binning latencies by cutoffs determined from the lat_mem_rd microbenchmark.'
        },
        {
            id: 'execution_latency',
            title: 'Execution Latency',
            description: 'The execution latency view identifies lines with high execution latency. The lines are sorted by total execution latency, and are reported with average execution latency and total count.'
        },
        {
            id: 'issue_latency',
            title: 'Issue Latency',
            description: 'The issue latency view identifies lines with high issue latency. The lines are sorted by total issue latency, and are reported with average issue latency and total count.'
        },
        {
            id: 'translation_latency',
            title: 'MMU Translation Latency',
            description: 'The translation latency view identifies lines with high translation latency. The lines are sorted by total translation latency, and are reported with average translation latency and total count.'
        }
    ];

    const branchConfigs = [
        {
            id: 'branch',
            title: 'Branch Misses',
            description: 'The branch miss view identifies conditional and indirect branch instructions. The instructions are sorted by count, and are reported with the branch misprediction rate and count.'
        }
    ];

    const buttonContainer = document.getElementById('button-container');
    const tableContent = document.getElementById('table-content');

    // Create Memory Operations group
    const memoryGroup = document.createElement('div');
    memoryGroup.className = 'button-group';
    const memoryLabel = document.createElement('h3');
    memoryLabel.textContent = 'Memory Operations';
    memoryGroup.appendChild(memoryLabel);

    memoryConfigs.forEach(config => {
        const button = document.createElement('button');
        button.textContent = config.title;
        button.className = 'table-button';
        button.addEventListener('click', (e) => {
            document.querySelectorAll('.table-button').forEach(btn => 
                btn.classList.remove('active'));
            button.classList.add('active');
            loadTable(config);
        });
        memoryGroup.appendChild(button);
    });

    // Create Branch Predictor group
    const branchGroup = document.createElement('div');
    branchGroup.className = 'button-group';
    const branchLabel = document.createElement('h3');
    branchLabel.textContent = 'Branch Predictor';
    branchGroup.appendChild(branchLabel);

    branchConfigs.forEach(config => {
        const button = document.createElement('button');
        button.textContent = config.title;
        button.className = 'table-button';
        button.addEventListener('click', (e) => {
            document.querySelectorAll('.table-button').forEach(btn => 
                btn.classList.remove('active'));
            button.classList.add('active');
            loadTable(config);
        });
        branchGroup.appendChild(button);
    });

    // Add groups to container
    buttonContainer.appendChild(memoryGroup);
    buttonContainer.appendChild(branchGroup);

    // Function to load table content using iframe
    function loadTable(config) {
        const contentWrapper = `
            <h2>${config.title}</h2>
            ${config.description ? `<p class="table-description">${config.description}</p>` : ''}
            <div class="table-container">
                <iframe src="data/js/${config.id}.html" 
                        frameborder="0" 
                        style="width: 100%; height: 500px;"
                        onload="this.height = this.contentWindow.document.documentElement.scrollHeight + 'px';">
                </iframe>
            </div>
        `;
        tableContent.innerHTML = contentWrapper;
    }

    // Load the first table by default and activate its button
    const firstButton = buttonContainer.querySelector('.table-button');
    if (firstButton) {
        firstButton.classList.add('active');
        loadTable(memoryConfigs[0]);
    }
});