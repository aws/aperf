let got_netstat_data = false;
let netstat_hide_zero_na_graphs = false;

let netstat_rules = {
    data_type: "netstat",
    pretty_name: "Netstat",
    rules: []
}

function getNetstatEntries(run, container_id, keys, run_data) {
    if (handleNoData(container_id, run_data)) return;

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
    
    const statsText = calculateStats(y_data);
    
    var TESTER = elem;
    var netstat_data: Partial<Plotly.PlotData> = {
        x: x_time,
        y: y_data,
        type: 'scatter',
    };
    let limits = key_limits.get(key);
    var layout = {
        title: createTitleWithStats(key, statsText),
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
    clear_and_create('netstat');
    form_graph_limits(netstat_raw_data);
    for (let i = 0; i < netstat_raw_data['runs'].length; i++) {
        let run_name = netstat_raw_data['runs'][i]['name'];
        let elem_id = `${run_name}-netstat-per-data`;
        let this_run_data = netstat_raw_data['runs'][i];
        getNetstatEntries(run_name, elem_id, this_run_data['keys'], this_run_data['key_values']);
    }
    got_netstat_data = true;
}
