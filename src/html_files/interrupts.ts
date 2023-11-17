let got_interrupt_data = false;

function getLine(run, elem, key, run_data) {
    var data = JSON.parse(run_data);
    var cpus = data[0].per_cpu.length;
    var interrupt_type_datas = [];
    for (let cpu = 0; cpu < cpus; cpu++) {
        var x_time = [];
        var y_data = [];
        data.forEach(function (value, index, arr) {
            value.per_cpu.forEach(function (v, i, a) {
                if (v.cpu == cpu) {
                    x_time.push(value.time.TimeDiff);
                    y_data.push(v.count);
                }
            })
        })
        var interrupt_cpu_data = {
            name: `CPU ${cpu}`,
            x: x_time,
            y: y_data,
            type: 'scatter',
        };
        interrupt_type_datas.push(interrupt_cpu_data);
    }
    var title;
    if (data[0].interrupt_device != "") {
        title = `Interrupt #${key} (${data[0].interrupt_device} ${data[0].interrupt_type})`;
    } else {
        title = `${key} (${data[0].interrupt_type})`;
    }
    var TESTER = elem;
    var layout = {
        title: title,
        xaxis: {
            title: 'Time (s)',
        },
        yaxis: {
            title: 'Count',
        }
    };
    Plotly.newPlot(TESTER, interrupt_type_datas, layout, { frameMargins: 0 });
}

function getLines(run, container_id, keys, run_data) {
    var data = keys;
    data.forEach(function (value, index, arr) {
        var elem = document.createElement('div');
        elem.id = `interrupt-${run}-${value}`;
        elem.style.float = "none";
        addElemToNode(container_id, elem);
        setTimeout(() => {
            getLine(run, elem, value, run_data[value]);
        }, 0);
    })
}

function interrupts() {
    if (got_interrupt_data) {
        return;
    }
    clearElements('interrupt-runs');
    runs_raw.forEach(function (value, index, arr) {
        // Run div
        var run_div = document.createElement('div');
        let this_run_data;
        run_div.id = `${value}-interrupts`;
        run_div.style.float = float_style;
        run_div.style.width = `${run_width}%`;
        addElemToNode('interrupt-runs', run_div);
        var run_node_id = run_div.id;
        // Show data
        var per_value_div = document.createElement('div');
        per_value_div.id = `${value}-interrupt-per-data`;
        addElemToNode(run_node_id, per_value_div);
        for (let i = 0; i < interrupts_raw_data['runs'].length; i++) {
            if (interrupts_raw_data['runs'][i]['name'] == value) {
                this_run_data = interrupts_raw_data['runs'][i];
                getLines(value, per_value_div.id, this_run_data['keys'], this_run_data['key_values']);
                break;
            }
        }
    })
    got_interrupt_data = true;
}
