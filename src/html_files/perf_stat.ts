let got_perf_stat_data = false;

function getEvents(run, container_id, keys, run_data) {
    if (keys.length == 0) {
        var no_data_div = document.createElement('div');
        no_data_div.id = `perfstat-${run}-nodata`;
        no_data_div.innerHTML = "No data collected";
        addElemToNode(container_id, no_data_div);
    } else {
        for (let i = 0; i < all_run_keys.length; i++) {
            let value = all_run_keys[i];
            var elem = document.createElement('div');
            elem.id = `perfstat-${run}-${value}`;
            elem.style.float = "none";
            addElemToNode(container_id, elem);
            emptyOrCallback(keys, false, getEvent, elem, value, run_data);
        }
    }
}

class StatValue {
    cpu: number;
    x_time: number[];
    y_data: number[];
}

function addData(perfstat_data, stat, timediff) {
    perfstat_data.forEach(function (value, index, arr) {
        if (value.cpu == stat.cpu) {
            value.x_time.push(timediff);
            value.y_data.push(stat.value)
        }
    })
}
function getEvent(elem, key, run_data) {
    var data = JSON.parse(run_data);
    var perfstat_datas = [];
    data.data[0].cpus.forEach(function (value, index, arr) {
        var cpu_stat = new StatValue();
        cpu_stat.cpu = value.cpu;
        cpu_stat.x_time = [];
        cpu_stat.y_data = [];
        perfstat_datas.push(cpu_stat);
    });
    data.data.forEach(function (value, index, arr) {
        value.cpus.forEach(function (stat, i_index, i_arr) {
            addData(perfstat_datas, stat, value.time.TimeDiff);
        })
    });
    var TESTER = elem;
    var end_datas = [];
    perfstat_datas.forEach(function (value, index, arr) {
        var cpu_string = "";
        if (value.cpu > -1) {
            cpu_string = `CPU ${value.cpu}`;
        }
        else {
            cpu_string = `Aggregate`;
        }
        var perfstat_line: Partial<Plotly.PlotData> = {
            name: cpu_string,
            x: value.x_time,
            y: value.y_data,
            type: 'scatter',
        };
        end_datas.push(perfstat_line);
    })
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
    Plotly.newPlot(TESTER, end_datas, layout, { frameMargins: 0 });
}

function perfStat() {
    if (got_perf_stat_data) {
        return;
    }
    clear_and_create('perfstat');
    form_graph_limits(perf_stat_raw_data);
    for (let i = 0; i < perf_stat_raw_data['runs'].length; i++) {
        let run_name = perf_stat_raw_data['runs'][i]['name'];
        let elem_id = `${run_name}-perfstat-per-data`;
        let this_run_data = perf_stat_raw_data['runs'][i];
        getEvents(run_name, elem_id, this_run_data['keys'], this_run_data['key_values']);
    }
    got_perf_stat_data = true;
}