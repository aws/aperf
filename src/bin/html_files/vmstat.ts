import './plotly.js';
import { clearElements, addElemToNode } from './index.js';
export { vmStat };

let got_data = false;

function getEntries(run, container_id) {
    const http = new XMLHttpRequest();
    http.onload = function () {
        var data = JSON.parse(http.response);
        data.forEach(function (value, index, arr) {
            var elem = document.createElement('div');
            elem.id = `vmstat-${run}-${value}`;
            elem.style.float = "none";
            addElemToNode(container_id, elem);
            getEntry(run, value, elem.id);
        })
    }
    http.open("GET", `visualize/vmstat?run=${run}&get=entries`);
    http.send();
}

function getEntry(run, key, parent_id) {
    const http = new XMLHttpRequest();
    http.onload = function () {
        var data = JSON.parse(http.response);
        var x_time = [];
        var y_data = [];
        data.forEach(function (value, index, arr) {
            x_time.push(value.time.TimeDiff);
            y_data.push(value.value);
        });
        var elem = document.createElement('div');
        elem.style.float = "none";
        addElemToNode(parent_id, elem);
        var TESTER = elem;
        var vmstat_data: Partial<Plotly.PlotData> = {
            x: x_time,
            y: y_data,
            type: 'scatter',
        };
        var layout = {
            title: `${key}`,
            xaxis: {
                title: 'Time (s)',
            },
            yaxis: {
                title: 'Pages',
            },
        }
        Plotly.newPlot(TESTER, [vmstat_data], layout, { frameMargins: 0 });
    }
    http.open("GET", `visualize/vmstat?run=${run}&get=values&key=${key}`);
    http.send();
}

function vmStat() {
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
        clearElements('vmstat-runs');
        data.forEach(function (value, index, arr) {
            // Run div
            var run_div = document.createElement('div');
            run_div.id = `${value}-vmstat`;
            run_div.style.float = float_style;
            run_div.style.width = `${run_width}%`;
            addElemToNode('vmstat-runs', run_div);
            var run_node_id = run_div.id;

            // Run name
            var h3_run_name = document.createElement('h3');
            h3_run_name.innerHTML = value;
            h3_run_name.style.textAlign = "center";
            addElemToNode(run_node_id, h3_run_name);

            // Show data
            var per_value_div = document.createElement('div');
            per_value_div.id = `${value}-vmstat-per-data`;
            addElemToNode(run_node_id, per_value_div);
            getEntries(value, per_value_div.id);
            got_data = true;
        })
    }
    http.open("GET", '/visualize/get?get=entries');
    http.send();
}
