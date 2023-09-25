let got_meminfo_data = false;
let TB = 1073741824;
let GB = 1048576;

let meminfo_average: Map<string, number> = new Map<string, number>();
function form_meminfo_averages() {
    runs_raw.forEach(function (value, index, arr) {
        let this_run_data;
        for (let i = 0; i < meminfo_raw_data['runs'].length; i++) {
            if (meminfo_raw_data['runs'][i]['name'] == value) {
                this_run_data = meminfo_raw_data['runs'][i];
                let keys = this_run_data['keys'];
                let values = this_run_data['key_values'];
                keys.forEach(function (v, i, a) {
                    var run_data = JSON.parse(values[v]);
                    let y_data = [];
                    run_data.data.values.forEach(function (rv, ri, ra) {
                        y_data.push(rv.value);
                    })
                    var total = 0;
                    for (i = 0; i < y_data.length; i++) {
                        total += y_data[i];
                    }
                    let average = total / y_data.length;
                    if (meminfo_average.has(v)) {
                        if (average > meminfo_average.get(v)) {
                            meminfo_average.set(v, average);
                        }
                    } else {
                        meminfo_average.set(v, average);
                    }
                });
            }
        }
    });
}

function get_divisor_unit(key) {
    var total = 0;
    let average = meminfo_average.get(key);
    if (average > TB) {
        return {
            divisor: TB,
            unit: "TB",
        };
    }
    if (average > GB) {
        return {
            divisor: GB,
            unit: "GB",
        };
    }
    return {
        divisor: 1,
        unit: "KB",
    };
}

function getMeminfo(elem, key, run_data) {
    var data = JSON.parse(run_data);
    var x_data = [];
    var y_data = [];
    data.data.values.forEach(function (value, index, arr) {
        x_data.push(value.time.TimeDiff);
        y_data.push(value.value);
    })

    var { divisor, unit } = get_divisor_unit(key);
    if (key.includes("Mem Total") ||
        key.includes("Vmalloc Total") ||
        key.includes("Hugepagesize")) {
        var mem_elem = document.createElement('h3');
        if (divisor == 1) {
            mem_elem.innerHTML = `${key}: ${y_data[0]}KB`;
        } else {
            mem_elem.innerHTML = `${key}: ${(y_data[0] / divisor).toLocaleString(undefined, { minimumFractionDigits: 0 })}${unit} (${(y_data[0]).toLocaleString(undefined, { minimumFractionDigits: 0 })}KB)`;
        }
        addElemToNode(elem.id, mem_elem);
        return;
    }
    if (key.includes("HugePages_")) {
        divisor = 1;
        unit = 'Count';
    }
    for (i = 0; i < y_data.length; i++) {
        y_data[i] /= divisor;
    }
    var meminfodata: Partial<Plotly.PlotData> = {
        x: x_data,
        y: y_data,
        type: 'scatter',
    };
    var TESTER = elem;
    let limits = key_limits.get(key);
    var layout = {
        title: key,
        xaxis: {
            title: 'Time (s)',
        },
        yaxis: {
            title: `${unit}`,
            range: [limits.low/divisor, limits.high/divisor],
        }
    };
    Plotly.newPlot(TESTER, [meminfodata], layout, { frameMargins: 0 });
}

function getMeminfoKeys(run, container_id, keys, run_data) {
    var data = keys;
    data.forEach(function (value, index, arr) {
        var elem = document.createElement('div');
        elem.id = `disk-stat-${run}-${value.name}`;
        elem.style.float = "none";
        addElemToNode(container_id, elem);
        setTimeout(() => {
            getMeminfo(elem, value, run_data[value]);
        }, 0);
    })
}

function meminfo() {
    if (got_meminfo_data) {
        return;
    }
    var data = runs_raw;
    var float_style = "none";
    if (data.length > 1) {
        float_style = "left";
    }
    var run_width = 100 / data.length;
    clearElements('meminfo-runs');
    form_meminfo_averages();
    form_graph_limits(meminfo_raw_data);
    data.forEach(function (value, index, arr) {
        // Run div
        var run_div = document.createElement('div');
        let this_run_data;
        run_div.id = `${value}-meminfo`;
        run_div.style.float = float_style;
        run_div.style.width = `${run_width}%`;
        addElemToNode('meminfo-runs', run_div);
        var run_node_id = run_div.id;

        // Run name
        var h3_run_name = document.createElement('h3');
        h3_run_name.innerHTML = value;
        h3_run_name.style.textAlign = "center";
        addElemToNode(run_node_id, h3_run_name);

        // Show data
        var per_value_div = document.createElement('div');
        per_value_div.id = `${value}-meminfo-per-data`;
        addElemToNode(run_node_id, per_value_div);
        for (let i = 0; i < meminfo_raw_data['runs'].length; i++) {
            if (meminfo_raw_data['runs'][i]['name'] == value) {
                this_run_data = meminfo_raw_data['runs'][i];
                getMeminfoKeys(value, per_value_div.id, this_run_data['keys'], this_run_data['key_values']);
            }
        }
    })
}