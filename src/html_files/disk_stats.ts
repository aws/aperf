let got_disk_stat_data = false;

function getStatValues(run, elem, key, run_data) {
    var disk_datas = [];
    var data = JSON.parse(run_data);
    data.data.forEach(function (v, i, a) {
        var x_time = [];
        var y_data = [];
        v.values.forEach(function (disk_value, disk_index, disk_arr) {
            x_time.push(disk_value.time.TimeDiff);
            y_data.push(disk_value.value);
        })
        var disk_data = {
            name: v.name,
            x: x_time,
            y: y_data,
            type: 'scatter',
        };
        disk_datas.push(disk_data);
    })
    var TESTER = elem;
    let unit;
    if (key.includes("Time")) {
        unit = 'Time (s)';
    } else if (key.includes("Sector")) {
        unit = 'Sectors (KB)';
    } else {
        unit = 'Count';
    }
    let limits = key_limits.get(key);
    var layout = {
        title: key,
        xaxis: {
            title: 'Time (s)',
        },
        yaxis: {
            title: unit,
            range: [limits.low, limits.high],
        },
    };
    Plotly.newPlot(TESTER, disk_datas, layout, { frameMargins: 0 });
}

function getStatKeys(run, container_id, mb, keys, run_data) {
    var data = keys;
    data.forEach(function (value, index, arr) {
        var elem = document.createElement('div');
        elem.id = `disk-stat-${run}-${value.name}`;
        elem.style.float = "none";
        addElemToNode(container_id, elem);
        setTimeout(() => {
            getStatValues(run, elem, value, run_data[value]);
        }, 0);
    })
}

function diskStats(mb: boolean) {
    if (got_disk_stat_data) {
        return;
    }
    var data = runs_raw;
    var float_style = "none";
    if (data.length > 1) {
        float_style = "left";
    }
    var run_width = 100 / data.length;
    clearElements('disk-stat-runs');
    form_graph_limits(disk_stats_raw_data);
    data.forEach(function (value, index, arr) {
        // Run div
        var run_div = document.createElement('div');
        let this_run_data;
        run_div.id = `${value}-disk-stat`;
        run_div.style.float = float_style;
        run_div.style.width = `${run_width}%`;
        addElemToNode('disk-stat-runs', run_div);
        var run_node_id = run_div.id;

        // Run name
        var h3_run_name = document.createElement('h3');
        h3_run_name.innerHTML = value;
        h3_run_name.style.textAlign = "center";
        addElemToNode(run_node_id, h3_run_name);

        // Show data
        var per_value_div = document.createElement('div');
        per_value_div.id = `${value}-disk-stat-per-data`;
        addElemToNode(run_node_id, per_value_div);
        for (let i = 0; i < disk_stats_raw_data['runs'].length; i++) {
            if (disk_stats_raw_data['runs'][i]['name'] == value) {
                this_run_data = disk_stats_raw_data['runs'][i];
                getStatKeys(value, per_value_div.id, mb, this_run_data['keys'], this_run_data['key_values']);
                break;
            }
        }
    })
    got_disk_stat_data = true;
}
