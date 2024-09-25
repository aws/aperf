let got_disk_stat_data = false;
let diskstat_hide_zero_na_graphs = false;
let diskstat_x_range: Map<string, Array<number>> = new Map<string, [0, 0]>();

function getStatValues(elem, key, run_data, run) {
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
    let x_range = getXRange(run);
    var layout = {
        title: key,
        xaxis: {
            title: 'Time (s)',
            range: [x_range[0], x_range[1]],
        },
        yaxis: {
            title: unit,
            range: [limits.low, limits.high],
        },
    };
    Plotly.newPlot(TESTER, disk_datas, layout, { frameMargins: 0 });
}

function getStatKeys(run, container_id, keys, run_data) {
    for (let i = 0; i < all_run_keys.length; i++) {
        let value = all_run_keys[i];
        var elem = document.createElement('div');
        elem.id = `disk-stat-${run}-${value}`;
        elem.style.float = "none";
        addElemToNode(container_id, elem);
        emptyOrCallback(keys, diskstat_hide_zero_na_graphs, getStatValues, elem, value, run_data, run);
    }
}

function diskStats(hide: boolean) {
    if (got_disk_stat_data && hide == diskstat_hide_zero_na_graphs && allRunXRangeUnchanged(diskstat_x_range)) {
        return;
    }
    diskstat_hide_zero_na_graphs = hide;
    clear_and_create('diskstat');
    form_graph_limits(disk_stats_raw_data);
    for (let i = 0; i < disk_stats_raw_data['runs'].length; i++) {
        let run_name = disk_stats_raw_data['runs'][i]['name']
        let elem_id = `${run_name}-diskstat-per-data`;
        let this_run_data = disk_stats_raw_data['runs'][i];
        diskstat_x_range.set(run_name, getXRange(run_name));
        getStatKeys(run_name, elem_id, this_run_data['keys'], this_run_data['key_values']);
    }
    got_disk_stat_data = true;
}
