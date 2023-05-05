let got_data = false;

class ProcessEntry {
    time: number;
    cpu_time: number;
}

let process_map = new Map<string, ProcessEntry[]>();

function getProcesses(run, container_id) {
    const http = new XMLHttpRequest();
    http.onload = function () {
        process_map = new Map<string, ProcessEntry[]>();
        var data = JSON.parse(http.response);
        data.forEach(function (value, index, arr) {
            value.entries.forEach(function(v, i, a) {
                let process_entry = new ProcessEntry();
                process_entry.time = value.time.TimeDiff;
                process_entry.cpu_time = v.cpu_time;
                let process_vec = process_map.get(v.name);
                if (process_vec == undefined) {
                    process_map.set(v.name, new Array<ProcessEntry>());
                    process_vec = process_map.get(v.name);
                }
                process_vec.push(process_entry);
                process_map.set(v.name, process_vec);
            })
        })
        console.log(process_map);
        let processes_data = [];
        for (let [key, value] of process_map) {
            var elem = document.createElement('div');
            elem.style.float = "none";
            addElemToNode(container_id, elem);
            var TESTER = elem;
            var x_time = [];
            var y_data = [];
            var prev_cpu_time = value[0].cpu_time;
            value.forEach(function (value, index, arr) {
                x_time.push(value.time);
                y_data.push(value.cpu_time - prev_cpu_time);
                prev_cpu_time = value.cpu_time;
            })
            var process_data = {
                name: key,
                x: x_time,
                y: y_data,
                type: 'scatter',
            };
            processes_data.push(process_data);
        }
        var layout = {
            title: 'Processes',
            xaxis: {
                title: 'Time(s)',
            },
            yaxis: {
                title: 'CPU Time (Ticks/s)',
            },
        }
        Plotly.newPlot(TESTER, processes_data, layout, { frameMargins: 0 });
    }
    http.open("GET", `visualize/processes?run=${run}&get=values`);
    http.send();
}

function processes() {
    if (got_data) {
        return;
    }
    const http = new XMLHttpRequest();
    http.onload = function () {
        var data = JSON.parse (http.response);
        var float_style = "none";
        if (data.length > 1) {
            float_style = "left";
        }
        var run_width = 100 / data.length;
        clearElements('processes-runs');
        data.forEach(function (value, index, arr) {
            var run_div = document.createElement('div');
            run_div.id = `${value}-processes`;
            run_div.style.float = float_style;
            run_div.style.width = `${run_width}%`;
            addElemToNode('processes-runs', run_div);

            var run_node_id = run_div.id;
            // Run name
            var h3_run_name = document.createElement('h3');
            h3_run_name.innerHTML = value;
            h3_run_name.style.textAlign = "center";
            addElemToNode(run_node_id, h3_run_name);

            // Show data
            var per_value_div = document.createElement('div');
            per_value_div.id = `${value}-process-per-data`;
            addElemToNode(run_node_id, per_value_div);
            getProcesses(value, per_value_div.id);
        })
        got_data = true;
    }
    http.open("GET", '/visualize/get?get=entries');
    http.send();
}