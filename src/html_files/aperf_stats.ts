let got_aperfstat_data = false;

let aperfstats_rules = {
    data_type: "aperf_run_stats",
    pretty_name: "Aperf Stats",
    rules: []
}

function getAperfEntry(elem, key, run_data) {
    var value = JSON.parse(run_data);
    let collect = value.collect;
    let print = value.print;
    let x_collect = [];
    let y_collect = [];
    let x_print = [];
    let y_print = [];
    for (var i = 0; i < collect.length; i++) {
        x_collect.push(collect[i].time.TimeDiff);
        y_collect.push(collect[i].time_taken);
    }
    for (var i = 0; i < print.length; i++) {
        x_print.push(print[i].time.TimeDiff);
        y_print.push(print[i].time_taken);
    }
    var TESTER = elem;
    var aperfstat_collect_data: Partial<Plotly.PlotData> = {
        name: `${key}-collect`,
        x: x_collect,
        y: y_collect,
        type: 'scatter',
    };
    var aperfstat_print_data: Partial<Plotly.PlotData> = {
        name: `${key}-print`,
        x: x_print,
        y: y_print,
        type: 'scatter',
    };
    let limits = key_limits.get(key);
    var layout = {
        title: `${key}`,
        xaxis: {
            title: 'Time (s)',
        },
        yaxis: {
            title: 'Time (us)',
            range: [limits.low, limits.high],
        },
    }
    Plotly.newPlot(TESTER, [aperfstat_collect_data, aperfstat_print_data], layout, { frameMargins: 0 });
}

function getAperfEntries(run, container_id, keys, run_data) {
    if (handleNoData(container_id, run_data)) return;

    for (let i = 0; i < all_run_keys.length; i++) {
        let value = all_run_keys[i];
        var elem = document.createElement('div');
        elem.id = `aperfstat-${run}-${value}`;
        elem.style.float = "none";
        addElemToNode(container_id, elem);
        emptyOrCallback(keys, false, getAperfEntry, elem, value, run_data);
    }
}

function aperfStat() {
    if (got_aperfstat_data) {
        return;
    }
    clear_and_create('aperfstat');
    form_graph_limits(aperf_stats_raw_data);
    for (let i = 0; i < aperf_stats_raw_data['runs'].length; i++) {
        let run_name = aperf_stats_raw_data['runs'][i]['name'];
        let elem_id = `${run_name}-aperfstat-per-data`;
        let this_run_data = aperf_stats_raw_data['runs'][i];
        getAperfEntries(run_name, elem_id, this_run_data['keys'], this_run_data['key_values']);
    }
    got_aperfstat_data = true;
}
