import './plotly.js';
import { clearElements, addElemToNode } from './index.js';
export { interrupts };

function getLine(run, key, elem) {
    const http = new XMLHttpRequest();
    http.onload = function() {
        var data = JSON.parse(http.response);
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
    http.open("GET", `visualize/interrupts?run=${run}&get=values&key=${key}`)
    http.send();
}

function getLines(run, container_id) {
    const http = new XMLHttpRequest();
    http.onload = function () {
        var data = JSON.parse(http.response);
        data.forEach(function (value, index, arr) {
            var elem = document.createElement('div');
            elem.id = `interrupt-${run}-${value}`;
            elem.style.float = "none";
            addElemToNode(container_id, elem);
            getLine(run, value, elem);
        })
    }
    http.open("GET", `visualize/interrupts?run=${run}&get=lines`);
    http.send();
}

function interrupts() {
    const http = new XMLHttpRequest();
    http.onload = function () {
        var data = JSON.parse(http.response);
        var float_style = "none";
        if (data.length > 1) {
            float_style = "left";
        }
        var run_width = 100 / data.length;
        clearElements('interrupt-runs');
        data.forEach(function (value, index, arr) {
            // Run div
            var run_div = document.createElement('div');
            run_div.id = `${value}-interrupts`;
            run_div.style.float = float_style;
            run_div.style.width = `${run_width}%`;
            addElemToNode('interrupt-runs', run_div);
            var run_node_id = run_div.id;

            // Run name
            var h3_run_name = document.createElement('h3');
            h3_run_name.innerHTML = value;
            h3_run_name.style.textAlign = "center";
            addElemToNode(run_node_id, h3_run_name);

            // Show data
            var per_value_div = document.createElement('div');
            per_value_div.id = `${value}-interrupt-per-data`;
            addElemToNode(run_node_id, per_value_div);
            getLines(value, per_value_div.id);
        })
    }
    http.open("GET", '/visualize/get?get=entries');
    http.send();
}
