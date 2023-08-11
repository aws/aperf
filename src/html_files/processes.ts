let got_process_data = false;

function getProcesses(run, run_data, container_id) {
    if (run_data.values == "No data collected") {
        var no_data_div = document.createElement('div');
        no_data_div.id = `processes-${run}-no-data`;
        no_data_div.innerHTML = "No data collected";
        addElemToNode(container_id, no_data_div);
        return;
    }
    var data = JSON.parse(run_data.values);
    data.end_entries.forEach(function (value, index, arr) {
        let process_datas = [];
        var x_time = [];
        var y_data = [];
        value.entries.forEach(function (v, i, a) {
            x_time.push(v.time.TimeDiff);
            y_data.push(v.cpu_time);
        });
        var process_data: Partial<Plotly.PlotData> = {
            name: `${value.name}`,
            x: x_time,
            y: y_data,
            type: 'scatter',
        };
        process_datas.push(process_data);
        var elem = document.createElement('div');
        elem.style.float = "none";
        addElemToNode(container_id, elem);
        var TESTER = elem;
        var layout = {
            title: value.name,
            xaxis: {
                title: 'Time(s)',
                range: [0, data.collection_time.TimeDiff],
            },
            yaxis: {
                title: 'Aggregate CPU Time (%)',
            },
        }
        Plotly.newPlot(TESTER, process_datas, layout, { frameMargins: 0 });
    })
}

function processes() {
    if (got_process_data) {
        return;
    }
    var data = runs_raw;
    var float_style = "none";
    if (data.length > 1) {
        float_style = "left";
    }
    var run_width = 100 / data.length;
    clearElements('processes-runs');
    data.forEach(function (value, index, arr) {
        var run_div = document.createElement('div');
        let this_run_data;
        run_div.id = `${value}-processes`;
        run_div.style.float = float_style;
        run_div.style.width = `${run_width}%`;
        addElemToNode('processes-runs', run_div);

        var run_node_id = run_div.id;
        // Run name
        var h3_run_name = document.createElement('h3');
        h3_run_name.innerHTML = value;
        h3_run_name.style.textAlign = "center";
        addElemToNode(run_node_id, h3_run_name);

        // Show data
        var per_value_div = document.createElement('div');
        per_value_div.id = `${value}-process-per-data`;
        addElemToNode(run_node_id, per_value_div);
        for (let i = 0; i < processes_raw_data['runs'].length; i++) {
            if (processes_raw_data['runs'][i]['name'] == value) {
                this_run_data = processes_raw_data['runs'][i];
                getProcesses(value, this_run_data['key_values'], per_value_div.id);
            }
        }
    })
    got_process_data = true;
}
