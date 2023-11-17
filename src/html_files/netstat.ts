let got_netstat_data = false;
let netstat_hide_zero_na_graphs = false;

function getNetstatEntries(run, container_id, keys, run_data) {
    for (let i = 0; i < all_run_keys.length; i++) {
        let value = all_run_keys[i];
        var elem = document.createElement('div');
        elem.id = `netstat-${run}-${value}`;
        elem.style.float = "none";
        addElemToNode(container_id, elem);
        emptyOrCallback(keys, netstat_hide_zero_na_graphs, getNetstatEntry, elem, value, run_data);
    }
}

function getNetstatEntry(elem, key, run_data) {
    var data = JSON.parse(run_data);
    var x_time = [];
    var y_data = [];
    data.data.forEach(function (value, index, arr) {
        x_time.push(value.time.TimeDiff);
        y_data.push(value.value);
    });
    var TESTER = elem;
    var netstat_data: Partial<Plotly.PlotData> = {
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
            title: 'Count',
            range: [limits.low, limits.high],
        },
    }
    Plotly.newPlot(TESTER, [netstat_data], layout, { frameMargins: 0 });
}

function netStat(hide: boolean) {
    if (got_netstat_data && hide == netstat_hide_zero_na_graphs) {
        return;
    }
    netstat_hide_zero_na_graphs = hide;
    clearElements('netstat-runs');
    form_graph_limits(netstat_raw_data);
    runs_raw.forEach(function (value, index, arr) {
        // Run div
        var run_div = document.createElement('div');
        let this_run_data;
        run_div.id = `${value}-netstat`;
        run_div.style.float = float_style;
        run_div.style.width = `${run_width}%`;
        addElemToNode('netstat-runs', run_div);
        var run_node_id = run_div.id;

        // Show data
        var per_value_div = document.createElement('div');
        per_value_div.id = `${value}-netstat-per-data`;
        addElemToNode(run_node_id, per_value_div);
        for (let i = 0; i < netstat_raw_data['runs'].length; i++) {
            if (netstat_raw_data['runs'][i]['name'] == value) {
                this_run_data = netstat_raw_data['runs'][i];
                getNetstatEntries(value, per_value_div.id, this_run_data['keys'], this_run_data['key_values']);
            }
        }
    })
    got_netstat_data = true;
}
