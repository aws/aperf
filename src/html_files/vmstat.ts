let got_vmstat_data = false;
let vmstat_hide_zero_na_graphs = false;

let vmstat_rules = {
    data_type: "vmstat",
    pretty_name: "Vmstat",
    rules: []
}

function getEntries(run, container_id, keys, run_data) {
    if (add_no_data_div(container_id, run_data)) {
        return;
    }
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
    clear_and_create('vmstat');
    form_graph_limits(vmstat_raw_data);
    for (let i = 0; i < vmstat_raw_data['runs'].length; i++) {
        let run_name = vmstat_raw_data['runs'][i]['name'];
        let elem_id = `${run_name}-vmstat-per-data`;
        let this_run_data = vmstat_raw_data['runs'][i];
        getEntries(run_name, elem_id, this_run_data['keys'], this_run_data['key_values']);
    }
    got_vmstat_data = true;
}
