let got_diskstats_data = false;
let diskstats_hide_zero_na_graphs = false;

let diskstats_rules = {
    data_type: "diskstats",
    pretty_name: "Disk Stats",
    rules: []
}

function getStatValues(elem, key, run_data) {
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

function getStatKeys(run, container_id, keys, run_data) {
    if (handleNoData(container_id, run_data)) return;
    
    for (let i = 0; i < all_run_keys.length; i++) {
        let value = all_run_keys[i];
        var elem = document.createElement('div');
        elem.id = `diskstats-${run}-${value}`;
        elem.style.float = "none";
        addElemToNode(container_id, elem);
        emptyOrCallback(keys, diskstats_hide_zero_na_graphs, getStatValues, elem, value, run_data);
    }
}

function diskStats(hide: boolean) {
    if (got_diskstats_data && hide == diskstats_hide_zero_na_graphs) {
        return;
    }
    diskstats_hide_zero_na_graphs = hide;
    clear_and_create('diskstats');
    form_graph_limits(diskstats_raw_data);
    for (let i = 0; i < diskstats_raw_data['runs'].length; i++) {
        let run_name = diskstats_raw_data['runs'][i]['name']
        let elem_id = `${run_name}-diskstats-per-data`;
        let this_run_data = diskstats_raw_data['runs'][i];
        getStatKeys(run_name, elem_id, this_run_data['keys'], this_run_data['key_values']);
    }
    got_diskstats_data = true;
}
