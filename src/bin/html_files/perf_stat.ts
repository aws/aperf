import './plotly.js';
import { clearElements, addElemToNode } from './index.js';
export { perfStat };

let got_data = false;

function getEvents(run, container_id) {
    const http = new XMLHttpRequest();
    http.onload = function () {
        if (http.response == "No data collected") {
            var no_data_div = document.createElement('div');
            no_data_div.id = `perfstat-${run}-nodata`;
            no_data_div.innerHTML = "No data collected";
            addElemToNode(container_id, no_data_div);
        } else {
            var data = JSON.parse(http.response);
            data.forEach(function (value, index, arr) {
                var elem = document.createElement('div');
                elem.id = `perfstat-${run}-${value}`;
                elem.style.float = "none";
                addElemToNode(container_id, elem);
                getEvent(run, value, elem.id);
            })
        }
    }
    http.open("GET", `visualize/perf_stat?run=${run}&get=events`);
    http.send();
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
function getEvent(run, key, parent_id) {
    const http = new XMLHttpRequest();
    http.onload = function () {
        var data = JSON.parse(http.response);
        var perfstat_datas = [];
        data[0].cpus.forEach(function (value, index, arr) {
            var cpu_stat = new StatValue();
            cpu_stat.cpu = value.cpu;
            cpu_stat.x_time = [];
            cpu_stat.y_data = [];
            perfstat_datas.push(cpu_stat);
        });
        data.forEach(function (value, index, arr) {
            value.cpus.forEach(function (stat, i_index, i_arr) {
                addData(perfstat_datas, stat, value.time.TimeDiff);
            })
        });
        var elem = document.createElement('div');
        elem.style.float = "none";
        addElemToNode(parent_id, elem);
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
        var layout = {
            title: `${key}`,
            xaxis: {
                title: 'Time (s)',
            },
            yaxis: {
                title: 'Count',
            },
        }
        Plotly.newPlot(TESTER, end_datas, layout, { frameMargins: 0 });
    }
    http.open("GET", `visualize/perf_stat?run=${run}&get=values&key=${key}`);
    http.send();
}

function perfStat() {
    if (got_data) {
        return;
    }
    const http = new XMLHttpRequest();
    http.onload = function () {
        var data = JSON.parse(http.response);
        var float_style = "none";
        if (data.length > 1) {
            float_style = "left";
        }
        var run_width = 100 / data.length;
        clearElements('perfstat-runs');
        data.forEach(function (value, index, arr) {
            // Run div
            var run_div = document.createElement('div');
            run_div.id = `${value}-perfstat`;
            run_div.style.float = float_style;
            run_div.style.width = `${run_width}%`;
            addElemToNode('perfstat-runs', run_div);
            var run_node_id = run_div.id;

            // Run name
            var h3_run_name = document.createElement('h3');
            h3_run_name.innerHTML = value;
            h3_run_name.style.textAlign = "center";
            addElemToNode(run_node_id, h3_run_name);

            // Show data
            var per_value_div = document.createElement('div');
            per_value_div.id = `${value}-perfstat-per-data`;
            addElemToNode(run_node_id, per_value_div);
            getEvents(value, per_value_div.id);
        })
        got_data = true;
    }
    http.open("GET", '/visualize/get?get=entries');
    http.send();
}
