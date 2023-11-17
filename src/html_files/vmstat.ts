let got_vmstat_data = false;
let vmstat_hide_zero_na_graphs = false;

function getEntries(run, container_id, keys, run_data) {
    for (let i = 0; i < all_run_keys.length; i++) {
        let value = all_run_keys[i];
        var elem = document.createElement('div');
        elem.id = `vmstat-${run}-${value}`;
        elem.style.float = "none";
        addElemToNode(container_id, elem);
        emptyOrCallback(keys, vmstat_hide_zero_na_graphs, getEntry, elem, value, run_data);
    }
}

function getEntry(elem, key, run_data) {
    var data = JSON.parse(run_data);
    var x_time = [];
    var y_data = [];
    data.data.forEach(function (value, index, arr) {
        x_time.push(value.time.TimeDiff);
        y_data.push(value.value);
    });
    var TESTER = elem;
    var vmstat_data: Partial<Plotly.PlotData> = {
        x: x_time,
        y: y_data,
        type: 'scatter',
    };
    let limits = key_limits.get(key);
    var layout = {
        title: `${key}`,
        xaxis: {
            title: 'Time (s)',
        },
        yaxis: {
            title: 'Pages',
            range: [limits.low, limits.high],
        },
    }
    Plotly.newPlot(TESTER, [vmstat_data], layout, { frameMargins: 0 });
}

function vmStat(hide: boolean) {
    if (got_vmstat_data && hide == vmstat_hide_zero_na_graphs) {
        return;
    }
    vmstat_hide_zero_na_graphs = hide;
    clearElements('vmstat-runs');
    form_graph_limits(vmstat_raw_data);
    runs_raw.forEach(function (value, index, arr) {
        // Run div
        var run_div = document.createElement('div');
        let this_run_data;
        run_div.id = `${value}-vmstat`;
        run_div.style.float = float_style;
        run_div.style.width = `${run_width}%`;
        addElemToNode('vmstat-runs', run_div);
        var run_node_id = run_div.id;

        // Show data
        var per_value_div = document.createElement('div');
        per_value_div.id = `${value}-vmstat-per-data`;
        addElemToNode(run_node_id, per_value_div);
        for (let i = 0; i < vmstat_raw_data['runs'].length; i++) {
            if (vmstat_raw_data['runs'][i]['name'] == value) {
                this_run_data = vmstat_raw_data['runs'][i];
                getEntries(value, per_value_div.id, this_run_data['keys'], this_run_data['key_values']);
            }
        }
    })
    got_vmstat_data = true;
}
