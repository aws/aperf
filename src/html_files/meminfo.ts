let got_meminfo_data = false;
let meminfo_hide_zero_na_graphs = false;
let TB = 1073741824;
let GB = 1048576;

let meminfo_rules = {
    data_type: "meminfo",
    pretty_name: "Meminfo",
    rules: []
}

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
        mem_elem.classList.add("extra");
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
    const originalStats = calculateStats(y_data);
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
        title: createTitleWithStats(key, originalStats),
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
    if (handleNoData(container_id, run_data)) return;

    for (let i = 0; i < all_run_keys.length; i++) {
        let value = all_run_keys[i];
        var elem = document.createElement('div');
        elem.id = `disk-stat-${run}-${value}`;
        elem.style.float = "none";
        addElemToNode(container_id, elem);
        emptyOrCallback(keys, meminfo_hide_zero_na_graphs, getMeminfo, elem, value, run_data);
    }
}

function meminfo(hide: boolean) {
    if (got_meminfo_data && hide == meminfo_hide_zero_na_graphs) {
        return;
    }
    meminfo_hide_zero_na_graphs = hide;
    clear_and_create('meminfo');
    form_meminfo_averages();
    form_graph_limits(meminfo_raw_data);
    for (let i = 0; i < meminfo_raw_data['runs'].length; i++) {
        let run_name = meminfo_raw_data['runs'][i]['name'];
        let elem_id = `${run_name}-meminfo-per-data`;
        let this_run_data = meminfo_raw_data['runs'][i];
        getMeminfoKeys(run_name, elem_id, this_run_data['keys'], this_run_data['key_values']);
    }
    got_meminfo_data = true;
}