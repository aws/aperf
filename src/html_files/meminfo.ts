let got_meminfo_data = false;
let TB = 1073741824;
let GB = 1048576;
function get_divisor_unit(values) {
    var total = 0;
    for (i = 0; i < values.length; i++) {
        total += values[i];
    }
    let average = total / values.length;
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
    data.values.forEach(function (value, index, arr) {
        x_data.push(value.time.TimeDiff);

        /* Bytes => kB */
        y_data.push(value.value / 1024);
    })

    var { divisor, unit } = get_divisor_unit(y_data);
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
    var layout = {
        title: key,
        xaxis: {
            title: 'Time (s)',
        },
        yaxis: {
            title: `${unit}`,
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