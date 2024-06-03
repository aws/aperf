let got_interrupt_data = false;
let interrupt_cpu_list: Map<string, CPUList> = new Map<string, CPUList>();

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
        var interrupt_cpu_data: Partial<Plotly.PlotData> = {
            name: `CPU ${cpu}`,
            x: x_time,
            y: y_data,
            type: 'scatter',
        };
        if (interrupt_cpu_list.get(run).cpulist.indexOf(cpu.toString()) == -1) {
            interrupt_cpu_data.visible = 'legendonly';
        }
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
    if (got_interrupt_data && allRunCPUListUnchanged(interrupt_cpu_list)) {
        return;
    }
    clear_and_create('interrupts');
    for (let i = 0; i < interrupts_raw_data['runs'].length; i++) {
        let run_name = interrupts_raw_data['runs'][i]['name'];
        interrupt_cpu_list.set(run_name, getCPUList(run_name));
        let elem_id = `${run_name}-interrupts-per-data`;
        let this_run_data = interrupts_raw_data['runs'][i];
        getLines(run_name, elem_id, this_run_data['keys'], this_run_data['key_values']);
    }
    got_interrupt_data = true;
}
