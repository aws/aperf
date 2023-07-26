let got_netstat_data = false;

function getNetstatEntries(run, container_id, keys, run_data) {
    var data = keys;
    data.forEach(function (value, index, arr) {
        var elem = document.createElement('div');
        elem.id = `netstat-${run}-${value}`;
        elem.style.float = "none";
        addElemToNode(container_id, elem);
        setTimeout(() => {
            getNetstatEntry(run, elem.id, value, run_data[value]);
        }, 0);
    })
}

function getNetstatEntry(run, parent_id, key, run_data) {
    var data = JSON.parse(run_data);
    var x_time = [];
    var y_data = [];
    data.forEach(function (value, index, arr) {
        x_time.push(value.time.TimeDiff);
        y_data.push(value.value);
    });
    var elem = document.createElement('div');
    elem.style.float = "none";
    addElemToNode(parent_id, elem);
    var TESTER = elem;
    var netstat_data: Partial<Plotly.PlotData> = {
        x: x_time,
        y: y_data,
        type: 'scatter',
    };
    var layout = {
        title: `${key}`,
        xaxis: {
            title: 'Time (s)',
        },
        yaxis: {
            title: 'Count',
        },
    }
    Plotly.newPlot(TESTER, [netstat_data], layout, { frameMargins: 0 });
}

function netStat() {
    if (got_netstat_data) {
        return;
    }
    var data = runs_raw;
    var float_style = "none";
    if (data.length > 1) {
        float_style = "left";
    }
    var run_width = 100 / data.length;
    clearElements('netstat-runs');
    data.forEach(function (value, index, arr) {
        // Run div
        var run_div = document.createElement('div');
        let this_run_data;
        run_div.id = `${value}-netstat`;
        run_div.style.float = float_style;
        run_div.style.width = `${run_width}%`;
        addElemToNode('netstat-runs', run_div);
        var run_node_id = run_div.id;

        // Run name
        var h3_run_name = document.createElement('h3');
        h3_run_name.innerHTML = value;
        h3_run_name.style.textAlign = "center";
        addElemToNode(run_node_id, h3_run_name);

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
    document.getElementById("netstat-loading").remove();
    got_netstat_data = true;
}
