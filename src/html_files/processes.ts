let got_process_data = false;

function getProcesses(run, container_id, run_data) {
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
    clear_and_create('processes');
    for (let i = 0; i < processes_raw_data['runs'].length; i++) {
        let run_name = processes_raw_data['runs'][i]['name'];
        let elem_id = `${run_name}-processes-per-data`;
        let this_run_data = processes_raw_data['runs'][i];
        getProcesses(run_name, elem_id, this_run_data['key_values']);
    }
    got_process_data = true;
}
